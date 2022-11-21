use frame_support::weights::Weight;

/// Weight functions needed for pallet extrinsics.
pub trait WeightInfo {
    fn create_pool(b: u32) -> Weight;
    fn add_liquidity(b: u32) -> Weight {
        sp_std::cmp::max_by(
            Self::add_liquidity_without_fee(b),
            Self::add_liquidity_with_fee(b),
            |x: _, y: _| x.ref_time().cmp(&y.ref_time())
        )
    }
    fn add_liquidity_without_fee(b: u32) -> Weight;
    fn add_liquidity_with_fee(b: u32) -> Weight;
    fn exchange() -> Weight;
    fn remove_liquidity(b: u32) -> Weight;
    fn remove_liquidity_imbalance(b: u32) -> Weight;
    fn remove_liquidity_one_coin() -> Weight;
    fn withdraw_admin_fees() -> Weight;
}

impl crate::WeightInfo for () {
    fn create_pool(_b: u32) -> Weight {
        Weight::zero()
    }
    fn add_liquidity_without_fee(_b: u32) -> Weight {
        Weight::zero()
    }
    fn add_liquidity_with_fee(_b: u32) -> Weight {
        Weight::zero()
    }
    fn exchange() -> Weight {
        Weight::zero()
    }
    fn remove_liquidity(_b: u32) -> Weight {
        Weight::zero()
    }
    fn remove_liquidity_imbalance(_b: u32) -> Weight {
        Weight::zero()
    }
    fn remove_liquidity_one_coin() -> Weight {
        Weight::zero()
    }
    fn withdraw_admin_fees() -> Weight {
        Weight::zero()
    }
}
