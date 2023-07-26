use frame_support::weights::Weight;

/// Weight functions needed for pallet extrinsics.
pub trait WeightInfo {
    fn create_pool(b: u32) -> Weight;
    fn add_liquidity(b: u32) -> Weight {
        Weight::from_parts(
            sp_std::cmp::max(
                Self::add_liquidity_without_fee(b).ref_time(),
                Self::add_liquidity_with_fee(b).ref_time(),
            ),
            0,
        )
    }
    fn add_liquidity_without_fee(b: u32) -> Weight;
    fn add_liquidity_with_fee(b: u32) -> Weight;
    fn exchange() -> Weight;
    fn remove_liquidity(b: u32) -> Weight;
    fn remove_liquidity_imbalance(b: u32) -> Weight;
    fn remove_liquidity_one_coin() -> Weight;
    fn withdraw_admin_fees() -> Weight;
    fn set_enable_state() -> Weight;
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
    fn set_enable_state() -> Weight {
        Weight::zero()
    }
}
