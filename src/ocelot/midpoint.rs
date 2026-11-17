//! Side-specific worst-mid model for raw locked/crossed Phoenix spline books.
//! No extra output haircut; the ordinary lot-boundary curve remains in use.
//! See MIDPOINT_VALIDATION.md. Normal books retain the native provider.
use super::{
    PHOENIX, State,
    codec::{Result, boolean, invalid, u64_at},
    curve::Curve,
    phoenix::NativePhoenix,
    quote::{Fill, Order, PhoenixQuoter},
    spline::{SideLimit, SplineDepth},
};
use solana_pubkey::Pubkey;
use std::sync::Arc;

pub(super) fn prepare(s: &State) -> Result<Arc<dyn PhoenixQuoter>> {
    let key = Pubkey::find_program_address(&[b"spline", s.config.book.as_ref()], &PHOENIX).0;
    let account = s
        .accounts
        .get(&key)
        .ok_or(invalid("missing spline account"))?;
    // Same saturating-expiry policy as the native slide-price check. Invalid
    // account data is an error, never a reason to try the approximation.
    let raw = SplineDepth::with_limits(account, s.config.book, s.clock.slot, true, |_, _| {
        Ok([SideLimit {
            lots: u64::MAX,
            skip: 0,
        }; 2])
    })?;
    let (Some(bid), Some(ask)) = (raw.best_bid(), raw.best_ask()) else {
        return Ok(Arc::new(NativePhoenix::prepare(s)?));
    };
    if bid < ask {
        return Ok(Arc::new(NativePhoenix::prepare(s)?));
    }
    // Use extrema from the validated raw parser, before risk/capacity limits.
    // A maker with live liquidity on either side contributes its midpoint.
    let (sell_mid, buy_mid) = raw
        .live_mid_bounds()
        .ok_or(invalid("locked spline book has no live maker midpoint"))?;
    let sell = Arc::new(
        recenter(s, key, sell_mid, bid.max(buy_mid))?.with_full_exit_price_bounds(bid, ask),
    );
    let buy = if buy_mid == sell_mid {
        sell.clone()
    } else {
        Arc::new(recenter(s, key, buy_mid, bid.max(buy_mid))?.with_full_exit_price_bounds(bid, ask))
    };
    log::debug!(
        "Ocelot worst-mid model: market={} bid={bid} ask={ask} buy_mid={buy_mid} sell_mid={sell_mid}",
        s.config.market
    );
    Ok(Arc::new(MidpointPhoenix {
        buy,
        sell,
        buy_for_direction: [s.market.long, s.position.base < 0],
    }))
}

fn recenter(s: &State, key: Pubkey, mid: u64, sell_fee_ticks: u64) -> Result<NativePhoenix> {
    let mut model = s.clone();
    let data = &mut model
        .accounts
        .get_mut(&key)
        .ok_or(invalid("missing spline model"))?
        .data;
    // The raw parser validated the header, count, owner and size above.
    let count = u32::from_le_bytes(data[72..76].try_into().unwrap()) as usize;
    for i in 0..count {
        let offset = 112 + i * 1336;
        if boolean(data, offset + 32)? && u64_at(data, offset + 40)? > 0 {
            data[offset + 40..offset + 48].copy_from_slice(&mid.to_le_bytes());
        }
    }
    // The venue retains the original State. Native validation, maker/taker
    // risk, FIFO reserves, fee and withdrawal checks run on this model, but
    // recentering does not certify their accuracy versus Phoenix slide prices.
    NativePhoenix::prepare_with_mint_fee_bound(&model, Some(sell_fee_ticks))
}

struct MidpointPhoenix {
    buy: Arc<NativePhoenix>,
    sell: Arc<NativePhoenix>,
    buy_for_direction: [bool; 2],
}
impl MidpointPhoenix {
    fn side(&self, buy: bool) -> &NativePhoenix {
        if buy { &self.buy } else { &self.sell }
    }
}
impl PhoenixQuoter for MidpointPhoenix {
    fn execution_limit(&self, order: &Order) -> Result<u64> {
        self.side(order.buy).execution_limit(order)
    }

    fn quote(&self, order: &Order) -> Result<Fill> {
        self.side(order.buy).quote(order)
    }
    fn curve(&self, mint: bool) -> Option<&Curve> {
        self.side(self.buy_for_direction[usize::from(!mint)])
            .curve(mint)
    }
    fn is_midpoint_approximation(&self) -> bool {
        true
    }
}
