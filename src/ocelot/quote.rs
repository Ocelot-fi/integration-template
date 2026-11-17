//! Native issuer arithmetic. No VM, RPC, account decoding or heap allocation.
//!
//! Normal books use the native execution model; locked/crossed books use the
//! side-specific worst-mid model. Validation scope: WORST_MID_VALIDATION.md.
use super::{
    State,
    codec::{Result, invalid},
};
use std::sync::Arc;

/// Prepared on each account refresh, never inside `quote()`.
pub trait PhoenixQuoteFactory: Send + Sync {
    fn prepare(&self, state: &State) -> Result<Arc<dyn PhoenixQuoter>>;
}

/// Immutable native matcher bound to exactly one State by its factory.
/// Implementations must be allocation-free and must not simulate or do I/O.
pub trait PhoenixQuoter: Send + Sync {
    /// Identifies the locked/crossed worst-mid path for diagnostics.
    /// This flag does not apply an output discount or certify capacity.
    fn is_midpoint_approximation(&self) -> bool {
        false
    }

    /// Model the full-fill IOC, including fees, funding, maker/taker risk,
    /// uncross reserves and collateral movements. Conservative providers must
    /// bound final issuer output, not merely individual book prices.
    /// Reject partial fills and unavailable Phoenix/Ember deposits/withdrawals.
    fn quote(&self, order: &Order) -> Result<Fill>;

    /// A prepared lower bound on this provider's MODELED issuer output, with a matching
    /// marginal price. Providers without a curve remain usable by the raw issuer
    /// kernel, but cannot publish discontinuous quotes through Titan's interface.
    fn curve(&self, _mint: bool) -> Option<&super::curve::Curve> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub lots: u64,
    pub buy: bool,
    pub reduce_only: bool,
    pub limit_ticks: u64,
    /// Exact collateral to deposit into the child BEFORE matching (zero on exits).
    pub deposit: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Fill {
    pub lots: u64,
    /// Modeled full-fill notional, excluding fees, in USDC atoms. The bundled
    /// provider uses a buy-cost upper bound / sell-proceeds lower bound.
    pub quote_atoms: u64,
    /// Fee on that execution model, with any conservative rounding reserve.
    /// Together with notional it must not overstate post-order equity.
    pub fee_atoms: u64,
    /// Conservative child collateral after the order, before withdrawal. On a
    /// full close it must equal the model's remaining equity (checked below).
    pub collateral_after: u64,
    /// Full-close collateral upper bound before fees. Used to reserve capacity
    /// when actual execution is better than the conservative output estimate.
    pub collateral_upper_bound: u64,
    /// Immediate Phoenix/Ember liquidity budget, independent of estimated PnL.
    pub withdrawal_capacity: u64,
    /// Immediately withdrawable and redeemable collateral, including Phoenix risk,
    /// queue/vault restrictions and Ember liquidity, for this post-order state.
    pub withdrawable_after: u64,
    /// Current cumulative funding index after matching; Ocelot requires stability.
    pub funding_after: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub cash: u64,
    pub min_venue_equity: u64,
    pub max_venue_leverage_bps: u32,
    pub withdrawal_capacity: u64,
}

impl Limits {
    pub fn load(s: &State) -> Result<Self> {
        use super::{PROGRAM_ID, codec::*, discriminator, global_bucket, instruction::pda};
        let pool = s.config.market;
        let read = |key| {
            s.accounts
                .get(&key)
                .ok_or(invalid("missing quote limit account"))
        };
        let f = account(
            read(pda(b"finance_v1", pool))?,
            PROGRAM_ID,
            &discriminator("account:MarketFinanceV1"),
            123,
        )?;
        let cash = u64_at(f, 51)?;
        let cash_active = boolean(f, 42)?;
        if !cash_active && cash != 0 {
            return Err(invalid("inactive cash accounting"));
        }
        if cash_active != (u64_at(&read(pool)?.data, 317)? & 1024 != 0) {
            return Err(invalid("cash activation mismatch"));
        }
        let vault = read(pda(b"market_cash_v1", pool))?;
        if vault.owner != spl_token::ID
            || vault.executable
            || vault.data.len() != 165
            || key(&vault.data, 0)? != s.market.usdc
            || key(&vault.data, 32)? != pool
            || u64_at(&vault.data, 64)? < cash
            || bytes::<4>(&vault.data, 72)? != [0; 4]
            || vault.data[108] != 1
            || bytes::<4>(&vault.data, 129)? != [0; 4]
        {
            return Err(invalid("invalid cash custody"));
        }
        let max_venue_leverage_bps = u32::from_le_bytes(bytes(f, 111)?);
        if max_venue_leverage_bps == 0 {
            return Err(invalid("invalid finance leverage"));
        }
        let g = account(
            read(global_bucket())?,
            PROGRAM_ID,
            &discriminator("account:GlobalWithdrawThrottleV1"),
            115,
        )?;
        if g[8] != 1 || key(g, 9)? == solana_pubkey::Pubkey::default() {
            return Err(invalid("invalid global withdrawal throttle"));
        }
        let global = capacity(g, 73, s.clock.unix_timestamp, true)?;
        let bucket = solana_pubkey::Pubkey::find_program_address(
            &[b"withdraw_throttle", b"market", pool.as_ref()],
            &PROGRAM_ID,
        )
        .0;
        let market = match s.accounts.get(&bucket) {
            None => u64::MAX, // An absent market bucket is explicitly unlimited on chain.
            Some(a) if a.owner == solana_sdk_ids::system_program::ID && a.data.is_empty() => {
                u64::MAX
            }
            Some(a) => {
                let d = account(
                    a,
                    PROGRAM_ID,
                    &discriminator("account:WithdrawThrottle"),
                    115,
                )?;
                if key(d, 40)? != pool || d[72] != 1 {
                    return Err(invalid("market throttle binding mismatch"));
                }
                capacity(d, 73, s.clock.unix_timestamp, false)?
            }
        };
        Ok(Self {
            cash,
            min_venue_equity: u64_at(f, 115)?,
            max_venue_leverage_bps,
            withdrawal_capacity: global.min(market),
        })
    }
}

fn capacity(d: &[u8], enabled: usize, now: i64, global: bool) -> Result<u64> {
    use super::codec::*;
    let enabled_flag = boolean(d, enabled)?;
    let limit = u64_at(d, enabled + 1)?;
    let mut window = i64_at(d, enabled + 9)?;
    let available = u64_at(d, enabled + 17)?;
    let mut updated = i64_at(d, enabled + 25)?;
    let remainder = u64_at(d, enabled + 33)?;
    if !global && window == 0 {
        window = 86_400;
    }
    if window <= 0
        || available > limit
        || remainder >= window as u64
        || (global && enabled_flag && limit == 0)
    {
        return Err(invalid("invalid withdrawal bucket"));
    }
    if !enabled_flag {
        return Ok(u64::MAX);
    }
    if !global && updated == 0 {
        updated = now;
    }
    if global && now < updated {
        return Err(invalid("withdrawal clock moved backwards"));
    }
    let elapsed = (now as i128 - updated as i128).max(0) as u128;
    let refill = (limit as u128)
        .checked_mul(elapsed)
        .and_then(|v| v.checked_add(remainder as u128))
        .ok_or(invalid("withdrawal refill overflow"))?
        / window as u128;
    Ok(available + refill.min((limit - available) as u128) as u64)
}

#[derive(Debug, Clone, Copy)]
pub struct Output {
    pub amount: u64,
    /// Output atoms per input atom. The raw issuer kernel returns its within-lot
    /// slope; Curve returns the active conservative segment's marginal rate.
    pub price: f64,
}

fn narrow(n: u128) -> Result<u64> {
    u64::try_from(n).map_err(|_| invalid("quote arithmetic overflow"))
}
fn fraction(n: u64, part: u64, whole: u64) -> Result<u64> {
    if whole == 0 || part > whole {
        return Err(invalid("invalid share fraction"));
    }
    narrow(n as u128 * part as u128 / whole as u128)
}
fn net(amount: u64, bps: u16) -> Result<u64> {
    amount
        .checked_sub(narrow(amount as u128 * bps as u128 / 10_000)?)
        .ok_or(invalid("invalid fee"))
}

fn venue_margin(s: &State, limits: Limits, base: i64, equity: u64) -> Result<()> {
    if base != 0
        && (equity == 0
            || equity < limits.min_venue_equity
            || (base.unsigned_abs() as u128)
                .checked_mul(s.mark.quote_per_lot)
                .and_then(|v| v.checked_mul(10_000))
                .ok_or(invalid("margin overflow"))?
                > equity as u128 * limits.max_venue_leverage_bps as u128)
    {
        return Err(invalid("finance venue margin exceeded"));
    }
    Ok(())
}

/// Shared with instruction construction, so execution cannot use a different lot.
pub fn order(s: &State, amount: u64, mint: bool) -> Result<Order> {
    order_with_limits(s, Limits::load(s)?, amount, mint)
}
fn order_with_limits(s: &State, limits: Limits, amount: u64, mint: bool) -> Result<Order> {
    let m = &s.market;
    if amount == 0 || s.mark.quote_per_lot == 0 {
        return Err(invalid("zero input or mark"));
    }
    let deposit = if mint {
        net(amount, m.mint_fee_bps)?
    } else {
        0
    };
    let lots = if mint {
        narrow(deposit as u128 * m.leverage as u128 / s.mark.quote_per_lot)?
    } else {
        if amount > s.actual_supply || m.shares == 0 {
            return Err(invalid("redemption exceeds supply"));
        }
        redemption_lots(s, limits, amount)?
    };
    if lots == 0 && mint {
        return Err(invalid("below minimum Phoenix lot"));
    }
    let buy = if mint { m.long } else { s.position.base < 0 };
    let limit_ticks = limit_ticks(s, buy)?;
    Ok(Order {
        lots,
        buy,
        reduce_only: !mint,
        limit_ticks,
        deposit,
    })
}

pub(crate) fn limit_ticks(s: &State, buy: bool) -> Result<u64> {
    narrow(if buy {
        s.mark.ticks as u128 * (10_000 + s.price_bps.min(100) as u128) / 10_000
    } else {
        (s.mark.ticks as u128 * (10_000 - s.price_bps.min(100) as u128)).div_ceil(10_000)
    })
}

/// V4 chooses the minimum whole-lot close that preserves the retained pool.
/// Mirrors withdrawal::plan, including cash-first payment and integer claims.
pub fn redemption_lots(s: &State, limits: Limits, amount: u64) -> Result<u64> {
    let supply = s.market.shares;
    let claim = fraction(s.gross, amount, supply)?;
    let fee = fraction(s.liability, amount, supply)?;
    if claim <= fee || limits.cash > s.gross || s.liability >= s.gross {
        return Err(invalid("no positive V4 holder claim"));
    }
    let base = s.position.base.unsigned_abs() as u128;
    if base == 0 {
        return Ok(0);
    }
    let net = s.gross - s.liability;
    let retained_net = net - (claim - fee);
    let venue = s.gross - limits.cash;
    let retained_venue = venue
        .checked_sub(claim.saturating_sub(limits.cash))
        .ok_or(invalid("invalid retained venue equity"))?;
    close_for_retained(s, limits, retained_net, retained_venue)
}

fn close_for_retained(
    s: &State,
    limits: Limits,
    retained_net: u64,
    retained_venue: u64,
) -> Result<u64> {
    let base = s.position.base.unsigned_abs() as u128;
    if base == 0 {
        return Ok(0);
    }
    let net = s.gross - s.liability;
    let venue = s.gross - limits.cash;
    let mark = s.mark.quote_per_lot;
    let pool_cap = (retained_net as u128
        * s.market.leverage as u128
        * (10_000 + s.market.rebalance_band_bps as u128)
        / 10_000
        / mark)
        .max(base * retained_net as u128 / net as u128);
    let venue_pro_rata = if venue == 0 {
        0
    } else {
        base * retained_venue as u128 / venue as u128
    };
    let venue_cap =
        (retained_venue as u128 * limits.max_venue_leverage_bps as u128 / 10_000 / mark)
            .max(venue_pro_rata);
    let mut keep = base.min(pool_cap).min(venue_cap);
    let collateral = funded_collateral(s)?;
    let unrealized = venue as i128 - collateral;
    if unrealized > 0 {
        keep = keep.min(base * retained_venue as u128 / unrealized as u128);
    }
    if retained_venue < limits.min_venue_equity {
        keep = keep
            .min(base - 1)
            .min(base * retained_net as u128 / net as u128)
            .min(venue_pro_rata);
    }
    narrow(base - keep)
}

pub(crate) fn funded_collateral(s: &State) -> Result<i128> {
    (s.position.base as i128)
        .checked_mul(s.mark.funding as i128 - s.position.funding as i128)
        .and_then(|f| (s.position.collateral as i128).checked_sub(f))
        .ok_or(invalid("funded collateral overflow"))
}

/// V4 can close the extra lot needed at the old V3 finance boundaries.
pub(crate) fn routing_redemption_cap(s: &State, _limits: Limits) -> Result<u64> {
    Ok(s.actual_supply)
}

/// Certify a contiguous routing prefix despite V4's live-mark risk islands.
/// Each interval uses worst-case rounded claims and close counts, not sampling.
/// Unproved intervals are subdivided; singleton quotes are checked exactly.
/// A bounded work budget returns the already certified prefix conservatively.
pub(crate) fn redemption_prefix(
    s: &State,
    limits: Limits,
    matcher: &dyn PhoenixQuoter,
    lo: u64,
    hi: u64,
    accepts: &dyn Fn(u64) -> bool,
) -> u64 {
    #[allow(clippy::too_many_arguments)]
    fn visit(
        s: &State,
        limits: Limits,
        matcher: &dyn PhoenixQuoter,
        a: u64,
        b: u64,
        accepts: &dyn Fn(u64) -> bool,
        budget: &mut usize,
        end: &mut u64,
    ) -> bool {
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        if a == b {
            if !accepts(a) {
                return false;
            }
            *end = b;
            return true;
        }
        if certify_redemption_interval(s, limits, matcher, a, b).unwrap_or(false)
            && accepts(a)
            && accepts(b)
        {
            *end = b;
            return true;
        }
        let mid = a + (b - a) / 2;
        visit(s, limits, matcher, a, mid, accepts, budget, end)
            && visit(s, limits, matcher, mid + 1, b, accepts, budget, end)
    }
    let mut end = lo;
    visit(s, limits, matcher, lo, hi, accepts, &mut 4096, &mut end);
    end
}
fn certify_redemption_interval(
    s: &State,
    limits: Limits,
    matcher: &dyn PhoenixQuoter,
    a: u64,
    b: u64,
) -> Result<bool> {
    let net = s.gross - s.liability;
    let venue = s.gross - limits.cash;
    let claim_lo = fraction(s.gross, a, s.market.shares)?;
    let claim_hi = fraction(s.gross, b, s.market.shares)?;
    let fee_lo = fraction(s.liability, a, s.market.shares)?;
    let fee_hi = fraction(s.liability, b, s.market.shares)?;
    let net_claim_lo = claim_lo.saturating_sub(fee_hi);
    let net_claim_hi = claim_hi
        .checked_sub(fee_lo)
        .ok_or(invalid("interval fee claim"))?;
    let retained_net = net
        .checked_sub(net_claim_hi)
        .ok_or(invalid("interval net claim"))?;
    let v_lo = venue
        .checked_sub(claim_hi.saturating_sub(limits.cash))
        .ok_or(invalid("interval venue claim"))?;
    let v_hi = venue
        .checked_sub(claim_lo.saturating_sub(limits.cash))
        .ok_or(invalid("interval venue claim"))?;
    let n_lo = close_for_retained(s, limits, net - net_claim_lo, v_hi)?;
    let n_hi = close_for_retained(s, limits, retained_net, v_lo)?;
    let base = s.position.base.unsigned_abs();
    let mut cost = 0u64;
    for n in [n_lo, n_hi] {
        let o = Order {
            lots: n,
            buy: s.position.base < 0,
            reduce_only: true,
            deposit: 0,
            limit_ticks: limit_ticks(s, s.position.base < 0)?,
        };
        let f = matcher.quote(&o)?;
        let marked = n as i128 * s.mark.quote_per_lot as i128;
        let loss = (if o.buy {
            f.quote_atoms as i128 - marked
        } else {
            marked - f.quote_atoms as i128
        }) + f.fee_atoms as i128;
        cost = cost.max(u64::try_from(loss.max(0)).map_err(|_| invalid("interval cost overflow"))?);
    }
    // A rounded fee can exceed the interpolation of its rounded endpoints by an atom.
    if n_lo != n_hi {
        cost = cost
            .checked_add(1)
            .ok_or(invalid("interval fee overflow"))?;
    }
    if net_claim_lo <= cost {
        return Ok(false);
    }
    let retained_venue = venue.saturating_sub(cost).min(v_lo);
    let remaining_max = base - n_lo;
    let remaining_min = base - n_hi;
    for mark in [
        s.mark.quote_per_lot,
        s.live_mark_bounds.0,
        s.live_mark_bounds.1,
    ] {
        let before_shift = mark_shift(s.position.base as i128, mark, s.mark.quote_per_lot)?;
        let shift = mark_shift(
            remaining_max as i128 * s.position.base.signum() as i128,
            mark,
            s.mark.quote_per_lot,
        )?
        .min(mark_shift(
            remaining_min as i128 * s.position.base.signum() as i128,
            mark,
            s.mark.quote_per_lot,
        )?);
        let bv = u64::try_from(
            (venue as i128)
                .checked_add(before_shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("interval negative venue"))?;
        let bn = u64::try_from(
            (net as i128)
                .checked_add(before_shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("interval negative net"))?;
        let av = u64::try_from(
            (retained_venue as i128)
                .checked_add(shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("interval negative venue"))?;
        let an = u64::try_from(
            (retained_net as i128)
                .checked_add(shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("interval negative net"))?;
        if !risk_at(s, limits, mark, remaining_max, bv, bn, av, an)? {
            return Ok(false);
        }
    }
    Ok(true)
}
#[allow(clippy::too_many_arguments)]
fn risk_at(
    s: &State,
    limits: Limits,
    mark: u128,
    base: u64,
    before_venue: u64,
    before_net: u64,
    venue: u64,
    net: u64,
) -> Result<bool> {
    if base == 0 {
        return Ok(true);
    }
    let original = s.position.base.unsigned_abs() as u128;
    let n = base as u128;
    let v_ok = before_venue > 0 && n * before_venue as u128 <= original * venue as u128;
    let p_ok = before_net > 0 && n * before_net as u128 <= original * net as u128;
    let notional = n
        .checked_mul(mark)
        .and_then(|v| v.checked_mul(10_000))
        .ok_or(invalid("interval exposure overflow"))?;
    Ok(venue > 0
        && net > 0
        && (notional <= venue as u128 * limits.max_venue_leverage_bps as u128 || v_ok)
        && (notional
            <= net as u128
                * s.market.leverage as u128
                * (10_000 + s.market.rebalance_band_bps as u128)
            || p_ok)
        && (venue >= limits.min_venue_equity || (n < original && v_ok && p_ok)))
}

fn mark_shift(base: i128, live: u128, captured: u128) -> Result<i128> {
    let live = i128::try_from(live).map_err(|_| invalid("live mark overflow"))?;
    let captured = i128::try_from(captured).map_err(|_| invalid("captured mark overflow"))?;
    live.checked_sub(captured)
        .and_then(|d| base.checked_mul(d))
        .ok_or(invalid("live equity shift overflow"))
}

fn retained_risk(
    s: &State,
    limits: Limits,
    base_after: i64,
    venue_after: u64,
    net_after: u64,
) -> Result<()> {
    let base = base_after.unsigned_abs() as u128;
    if base == 0 {
        return Ok(());
    }
    let original = s.position.base.unsigned_abs() as u128;
    if base > original || venue_after == 0 || net_after == 0 {
        return Err(invalid("invalid retained V4 backing"));
    }
    for mark in [
        s.mark.quote_per_lot,
        s.live_mark_bounds.0,
        s.live_mark_bounds.1,
    ] {
        let before_shift = mark_shift(s.position.base as i128, mark, s.mark.quote_per_lot)?;
        let after_shift = mark_shift(base_after as i128, mark, s.mark.quote_per_lot)?;
        let before_venue = u64::try_from(
            ((s.gross - limits.cash) as i128)
                .checked_add(before_shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("negative live pre-exit equity"))?;
        let before_net = u64::try_from(
            ((s.gross - s.liability) as i128)
                .checked_add(before_shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("negative live pre-exit net"))?;
        let live_venue = u64::try_from(
            (venue_after as i128)
                .checked_add(after_shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("negative live retained equity"))?;
        let live_net = u64::try_from(
            (net_after as i128)
                .checked_add(after_shift)
                .ok_or(invalid("live equity overflow"))?,
        )
        .map_err(|_| invalid("negative live retained net"))?;
        let venue_non_worsening =
            before_venue > 0 && base * before_venue as u128 <= original * live_venue as u128;
        let pool_non_worsening =
            before_net > 0 && base * before_net as u128 <= original * live_net as u128;
        let notional = base
            .checked_mul(mark)
            .and_then(|v| v.checked_mul(10_000))
            .ok_or(invalid("retained notional overflow"))?;
        if live_venue == 0
            || live_net == 0
            || (notional > live_venue as u128 * limits.max_venue_leverage_bps as u128
                && !venue_non_worsening)
            || (notional
                > live_net as u128
                    * s.market.leverage as u128
                    * (10_000 + s.market.rebalance_band_bps as u128)
                && !pool_non_worsening)
            || (live_venue < limits.min_venue_equity
                && !(base < original && venue_non_worsening && pool_non_worsening))
        {
            return Err(invalid("V4 retained risk exceeded"));
        }
    }
    Ok(())
}

pub fn quote(
    s: &State,
    limits: Limits,
    matcher: &dyn PhoenixQuoter,
    amount: u64,
    mint: bool,
) -> Result<Output> {
    let m = &s.market;
    if m.paused || (mint && m.deposits_paused) {
        return Err(invalid("market paused"));
    }
    if s.position.base != 0 && (s.position.base > 0) != m.long {
        return Err(invalid("position direction mismatch"));
    }
    let equity = s.gross.saturating_sub(s.liability);
    if m.shares == 0 {
        if s.gross != 0 || s.position.collateral != 0 || s.position.base != 0 || s.liability != 0 {
            return Err(invalid("zero supply requires recovery"));
        }
        if !mint {
            return Err(invalid("no shares to redeem"));
        }
    } else if equity == 0 {
        return Err(invalid("no holder equity"));
    }
    let ratio = if m.shares == 0 {
        1.0
    } else {
        m.shares as f64 / equity as f64
    };
    let price = if mint {
        ratio * (1.0 - m.mint_fee_bps as f64 / 10_000.0)
    } else {
        (1.0 - m.redeem_fee_bps as f64 / 10_000.0) / ratio
    };
    if amount == 0 {
        return Ok(Output { amount: 0, price });
    }
    let order = order_with_limits(s, limits, amount, mint)?;
    if mint {
        if m.deposit_cap != 0 && amount > m.deposit_cap {
            return Err(invalid("deposit cap exceeded"));
        }
        if m.equity_cap != 0 && equity as u128 + order.deposit as u128 > m.equity_cap as u128 {
            return Err(invalid("equity cap exceeded"));
        }
    }
    let claim = if mint {
        0
    } else {
        fraction(s.gross, amount, m.shares)?
    };
    if claim > limits.withdrawal_capacity {
        return Err(invalid("withdrawal capacity exceeded"));
    }
    // Even a cash-only exit needs the provider's withdrawal availability check.
    let fill = matcher.quote(&order)?;
    if fill.lots != order.lots || fill.funding_after != s.mark.funding {
        return Err(invalid("incomplete fill or funding changed"));
    }
    if order.lots == 0 && (fill.quote_atoms != 0 || fill.fee_atoms != 0) {
        return Err(invalid("nonzero fill on empty order"));
    }
    let limit_quote = order.lots as u128 * order.limit_ticks as u128;
    let limit_quote = limit_quote
        .checked_mul(s.config.tick_size as u128)
        .ok_or(invalid("limit overflow"))?;
    if (order.buy && fill.quote_atoms as u128 > limit_quote)
        || (!order.buy && (fill.quote_atoms as u128) < limit_quote)
    {
        return Err(invalid("fill exceeds IOC price limit"));
    }
    let marked = (order.lots as u128)
        .checked_mul(s.mark.quote_per_lot)
        .ok_or(invalid("notional overflow"))?;
    let marked = i128::try_from(marked).map_err(|_| invalid("notional overflow"))?;
    let delta = if order.buy {
        marked - fill.quote_atoms as i128
    } else {
        fill.quote_atoms as i128 - marked
    }
    .checked_sub(fill.fee_atoms as i128)
    .ok_or(invalid("equity overflow"))?;
    let after = (s.gross as i128)
        .checked_add(order.deposit as i128)
        .and_then(|v| v.checked_add(delta))
        .and_then(|v| u64::try_from(v).ok())
        .ok_or(invalid("negative or overflowing equity"))?;
    let base_after = s.position.base as i128
        + if order.buy {
            order.lots as i128
        } else {
            -(order.lots as i128)
        };
    let base_after = i64::try_from(base_after).map_err(|_| invalid("position overflow"))?;
    let venue_after = after
        .checked_sub(limits.cash)
        .ok_or(invalid("negative venue equity"))?;
    let output = if mint {
        venue_margin(s, limits, base_after, venue_after)?;
        for mark in [s.live_mark_bounds.0, s.live_mark_bounds.1] {
            let shift = mark_shift(base_after as i128, mark, s.mark.quote_per_lot)?;
            let live_equity = u64::try_from(
                (venue_after as i128)
                    .checked_add(shift)
                    .ok_or(invalid("live equity overflow"))?,
            )
            .map_err(|_| invalid("negative post-trade mark equity"))?;
            if live_equity < limits.min_venue_equity
                || (base_after.unsigned_abs() as u128)
                    .checked_mul(mark)
                    .and_then(|n| n.checked_mul(10_000))
                    .ok_or(invalid("live exposure overflow"))?
                    > live_equity as u128 * limits.max_venue_leverage_bps as u128
            {
                return Err(invalid("post-trade mark exceeds mint finance bounds"));
            }
        }
        let collateral_added = u64::try_from(fill.collateral_after as i128 - funded_collateral(s)?)
            .map_err(|_| invalid("no contributed collateral"))?;
        let added = after
            .checked_sub(s.gross)
            .map(|v| v.min(collateral_added))
            .filter(|v| *v > 0 && *v <= order.deposit)
            .ok_or(invalid("no added equity"))?;
        let shares = if m.shares == 0 {
            added
        } else {
            narrow(added as u128 * m.shares as u128 / equity as u128)?
        };
        if shares == 0 {
            return Err(invalid("zero shares"));
        }
        if m.shares != 0 {
            // With at least 10,000 output atoms, any sub-atom share rounding is
            // within the on-chain 1bp bound, including a better actual fill.
            if shares < 10_000 {
                return Err(invalid("mint too small to bound share rounding"));
            }
            let entitlement = added as u128 * m.shares as u128;
            if entitlement - shares as u128 * equity as u128 > entitlement / 10_000 {
                return Err(invalid("mint rounding exceeds one basis point"));
            }
        }
        m.shares
            .checked_add(shares)
            .ok_or(invalid("share supply overflow"))?;
        shares
    } else {
        let cash_claim = limits.cash.min(claim);
        let maximum_withdrawal = if amount == m.shares {
            fill.collateral_upper_bound
        } else {
            claim.saturating_sub(cash_claim)
        };
        let maximum_proceeds = maximum_withdrawal
            .checked_add(cash_claim)
            .ok_or(invalid("withdrawal upper bound overflow"))?;
        if maximum_withdrawal > fill.withdrawal_capacity
            || maximum_proceeds > limits.withdrawal_capacity
        {
            return Err(invalid("insufficient capacity for better actual execution"));
        }
        let proceeds = if amount == m.shares {
            if base_after != 0 {
                return Err(invalid("full redemption left exposure"));
            }
            let proceeds = fill
                .collateral_after
                .checked_add(limits.cash)
                .ok_or(invalid("proceeds overflow"))?;
            if proceeds != after {
                return Err(invalid("full close left non-collateral equity"));
            }
            proceeds
        } else {
            claim.min(
                after
                    .checked_sub(s.gross - claim)
                    .ok_or(invalid("execution cost exceeds claim"))?,
            )
        };
        if proceeds == 0 || proceeds > limits.withdrawal_capacity {
            return Err(invalid("no withdrawal capacity"));
        }
        let cash_payout = limits.cash.min(proceeds);
        let withdrawal = proceeds - cash_payout;
        if withdrawal > fill.collateral_after || withdrawal > fill.withdrawable_after {
            return Err(invalid("Phoenix or Ember withdrawal unavailable"));
        }
        let liability = fraction(s.liability, amount, m.shares)?;
        if proceeds <= liability {
            return Err(invalid("V4 exit impairs allocated fees"));
        }
        let remaining_gross = after
            .checked_sub(proceeds)
            .ok_or(invalid("negative remaining gross"))?;
        let remaining_net = remaining_gross
            .checked_sub(s.liability - liability)
            .ok_or(invalid("negative retained net equity"))?;
        retained_risk(
            s,
            limits,
            base_after,
            venue_after
                .checked_sub(withdrawal)
                .ok_or(invalid("negative remaining venue equity"))?,
            remaining_net,
        )?;
        net(proceeds.saturating_sub(liability), m.redeem_fee_bps)?
    };
    if output == 0 {
        return Err(invalid("zero output"));
    }
    Ok(Output {
        amount: output,
        price,
    })
}
