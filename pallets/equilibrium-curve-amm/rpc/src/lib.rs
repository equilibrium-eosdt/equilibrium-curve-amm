use std::sync::Arc;
use std::convert::TryInto;
use codec::Codec;
use sp_blockchain::HeaderBackend;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use sp_runtime::{generic::BlockId, traits::{Block as BlockT, MaybeDisplay}};
use sp_api::ProvideRuntimeApi;
use sp_rpc::number::NumberOrHex;
use equilibrium_curve_amm_rpc_runtime_api::{PoolTokenIndex, PoolId};
pub use equilibrium_curve_amm_rpc_runtime_api::EquilibriumCurveAmmApi as EquilibriumCurveAmmRuntimeApi;

#[rpc]
pub trait EquilibriumCurveAmmApi<Balance> {
    #[rpc(name = "equilibriumCurveAmm_getDy")]
    fn get_dy(
        &self,
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: Balance
    ) -> Result<Option<Balance>>;

    #[rpc(name = "equilibriumCurveAmm_getWithdrawOneCoin")]
    fn get_withdraw_one_coin(
        &self,
        pool_id: PoolId,
        burn_amount: Balance,
        i: PoolTokenIndex,
    ) -> Result<Option<Balance>>;

}

pub struct EquilibriumCurveAmm<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> EquilibriumCurveAmm<C, P> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client, _marker: Default::default() }
    }
}

impl<C, Block, Balance> EquilibriumCurveAmmApi<Balance> for EquilibriumCurveAmm<C, Block>
    where
        Block: BlockT,
        C: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
        C::Api: EquilibriumCurveAmmRuntimeApi<Block, Balance>,
        Balance: Codec + MaybeDisplay + Copy + TryInto<NumberOrHex>,
{
    fn get_dy(
        &self,
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: Balance
    ) -> Result<Option<Balance>> {
        let at = BlockId::hash(self.client.info().best_hash);
        let api = self.client.runtime_api();
        let dy = api.get_dy(&at, pool_id, i, j, dx).ok().flatten();
        Ok(dy)
    }

    fn get_withdraw_one_coin(
        &self,
        pool_id: PoolId,
        burn_amount: Balance,
        i: PoolTokenIndex
    ) -> Result<Option<Balance>> {
        let at = BlockId::hash(self.client.info().best_hash);
        let api = self.client.runtime_api();
        let dy = api.get_withdraw_one_coin(&at, pool_id, burn_amount, i).ok().flatten();
        Ok(dy)
    }
}