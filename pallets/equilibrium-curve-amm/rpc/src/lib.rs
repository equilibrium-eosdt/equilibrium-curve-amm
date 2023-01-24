use codec::{Codec, Decode, Encode};
pub use equilibrium_curve_amm_rpc_runtime_api::EquilibriumCurveAmmApi as EquilibriumCurveAmmRuntimeApi;
use equilibrium_curve_amm_rpc_runtime_api::{PoolId, PoolTokenIndex};
use jsonrpsee::{
    core::{async_trait, Error as RpcError, RpcResult},
    proc_macros::rpc,
};
use sp_api::{CallApiAt, CallApiAtParams, ExecutionContext, NativeOrEncoded, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay},
};
use std::sync::Arc;
use std::{convert::TryInto, fmt::Debug};

#[rpc(client, server)]
pub trait EquilibriumCurveAmmApi<Balance, Hash> {
    #[method(name = "equilibriumCurveAmm_getDy")]
    fn get_dy(
        &self,
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: Balance,
    ) -> RpcResult<Option<Balance>>;

    #[method(name = "equilibriumCurveAmm_getWithdrawOneCoin")]
    fn get_withdraw_one_coin(
        &self,
        pool_id: PoolId,
        burn_amount: Balance,
        i: PoolTokenIndex,
    ) -> RpcResult<Option<Balance>>;
    #[method(name = "equilibriumCurveAmm_getVirtualPrice")]
    fn get_virtual_price(&self, pool_id: PoolId, block: Option<Hash>)
        -> RpcResult<Option<Balance>>;
}

pub struct EquilibriumCurveAmm<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> EquilibriumCurveAmm<C, P> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

#[async_trait]
impl<C, Block, Balance> EquilibriumCurveAmmApiServer<Balance, Block::Hash>
    for EquilibriumCurveAmm<C, Block>
where
    Block: BlockT,
    C: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C: CallApiAt<Block>,
    C::Api: EquilibriumCurveAmmRuntimeApi<Block, Balance>,
    Balance: Codec + MaybeDisplay + Copy + TryInto<NumberOrHex> + PartialEq + Debug,
{
    fn get_dy(
        &self,
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: Balance,
    ) -> RpcResult<Option<Balance>> {
        let at = BlockId::hash(self.client.info().best_hash);
        let api = self.client.runtime_api();
        let dy = api.get_dy(&at, pool_id, i, j, dx).ok().flatten();
        Ok(dy)
    }

    fn get_withdraw_one_coin(
        &self,
        pool_id: PoolId,
        burn_amount: Balance,
        i: PoolTokenIndex,
    ) -> RpcResult<Option<Balance>> {
        let at = BlockId::hash(self.client.info().best_hash);
        let api = self.client.runtime_api();
        let dy = api
            .get_withdraw_one_coin(&at, pool_id, burn_amount, i)
            .ok()
            .flatten();
        Ok(dy)
    }

    fn get_virtual_price(
        &self,
        pool_id: PoolId,
        mb_block: Option<Block::Hash>,
    ) -> RpcResult<Option<Balance>> {
        if let Some(block) = mb_block {
            let at = BlockId::hash(block);
            let native_or_encoded: NativeOrEncoded<Option<Balance>> = self
                .client
                .call_api_at(CallApiAtParams::<_, fn() -> _, _> {
                    at: &&at,
                    function: "EquilibriumCurveAmmApi_get_virtual_price",
                    native_call: None,
                    arguments: Encode::encode(&pool_id),
                    overlayed_changes: &Default::default(),
                    storage_transaction_cache: &Default::default(),
                    context: ExecutionContext::BlockConstruction,
                    recorder: &None,
                })
                .map_err(|e| RpcError::Custom(e.to_string()))?;

            let virtual_price = match native_or_encoded {
                NativeOrEncoded::Native(native) => native,
                NativeOrEncoded::Encoded(encoded) => {
                    let decoded: Option<Balance> = Decode::decode(&mut &encoded[..])
                        .map_err(|e| RpcError::Custom(e.to_string()))?;
                    decoded
                }
            };

            Ok(virtual_price)
        } else {
            let at = BlockId::hash(self.client.info().best_hash);
            let api = self.client.runtime_api();
            let virtual_price = api.get_virtual_price(&at, pool_id).ok().flatten();
            Ok(virtual_price)
        }
    }
}
