#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod nft_module;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct ExampleAttributes {
    pub creation_timestamp: u64,
}

#[elrond_wasm::contract]
pub trait NftMinter: nft_module::NftModule {
    #[init]
    fn init(&self) {}

    #[allow(clippy::too_many_arguments)]
    #[payable("EGLD")]
    #[endpoint(mintNFT)]
    fn mint_nft(
        &self,
        royalties: BigUint,
        uri: ManagedBuffer
    ) -> SCResult<u64> {

        let caller = self.blockchain().get_caller();
        let user_balance = self.blockchain().get_balance(&caller);
          let price:u64 = 50000000000000000u64; // selling price

        require!(
            user_balance < price,
            "You don't have enough EGLD to mint"
        );

        let attributes = ExampleAttributes {
            creation_timestamp: self.blockchain().get_block_timestamp(),
        };
        self.mint_nft_with_attributes(
            royalties,
            attributes,
            uri
        )
    }
}
