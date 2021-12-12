elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use elrond_wasm::elrond_codec::TopEncode;
use elrond_wasm::types::TokenIdentifier;


const NFT_AMOUNT: u32 = 1; // minting amount of NFT : for now users can mint 1 at once
const SELLING_PRICE: u64 = 350000000000000000; // selling price  0.05 EGLD  0.35
const ROYALTIES_MAX: u32 = 5_000; // 300  3%  400

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct PriceTag<M: ManagedTypeApi> {
    pub token: TokenIdentifier<M>,
    pub nonce: u64,
    pub amount: BigUint<M>,
}

#[elrond_wasm::module]
pub trait NftModule {
    // endpoints - owner-only

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(&self) -> SCResult<AsyncCall> {
        require!(self.nft_token_id().is_empty(), "Token already issued");

        Ok(self
            .send()
            .esdt_system_sc_proxy()
            .issue_non_fungible(
                BigUint::from(SELLING_PRICE),
                &ManagedBuffer::new_from_bytes(b"BabiesDegenApe"),
                &ManagedBuffer::new_from_bytes(b"BDAPE"),
                NonFungibleTokenProperties {
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_change_owner: false,
                    can_upgrade: false,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().issue_callback()))
    }

    #[only_owner]
    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self) -> SCResult<AsyncCall> {
        self.require_token_issued()?;

        Ok(self
            .send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &self.nft_token_id().get(),
                (&[EsdtLocalRole::NftCreate][..]).into_iter().cloned(),
            )
            .async_call())
    }

    // views

    #[allow(clippy::type_complexity)]
    #[view(getNftPrice)]
    fn get_nft_price(
        &self,
        nft_nonce: u64,
    ) -> OptionalResult<MultiResult3<TokenIdentifier, u64, BigUint>> {
        if self.price_tag(nft_nonce).is_empty() {
            // NFT was already sold
            OptionalResult::None
        } else {
            let price_tag = self.price_tag(nft_nonce).get();

            OptionalResult::Some((price_tag.token, price_tag.nonce, price_tag.amount).into())
        }
    }

    // callbacks

    #[callback]
    fn issue_callback(&self, #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.nft_token_id().set(&token_id);
            },
            ManagedAsyncCallResult::Err(_) => {
                let caller = self.blockchain().get_owner_address();
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    self.send()
                        .direct(&caller, &token_id, 0, &returned_tokens, &[]);
                }
            },
        }
    }

    // private

    #[allow(clippy::too_many_arguments)]


    fn mint_nft_with_attributes<T: TopEncode>(
        &self,
        royalties: BigUint,
        attributes: T,
        uri: ManagedBuffer,
    ) -> SCResult<u64> {
        require!(royalties <= ROYALTIES_MAX, "Royalties cannot exceed 100%");
        let nft_token_id = self.nft_token_id().get();

        let mut serialized_attributes = Vec::new();
        attributes.top_encode(&mut serialized_attributes)?;

        let attributes_hash = self.crypto().sha256(&serialized_attributes);
        let hash_buffer = ManagedBuffer::from(attributes_hash.as_bytes());

        let mut uris = ManagedVec::new();
        
        uris.push(uri);

        let nft_nonce = self.send().esdt_nft_create(
            &nft_token_id,
            &BigUint::from(NFT_AMOUNT),
            &ManagedBuffer::new_from_bytes(b"BDAPE"),
            &royalties,
            &hash_buffer,
            &attributes,
            &uris,
        );

        self.price_tag(nft_nonce).set(&PriceTag {
            token: TokenIdentifier::egld(),
            nonce: 0,
            amount: BigUint::from(SELLING_PRICE),
        });

        self.price_tag(nft_nonce).clear();

        let nft_token_id = self.nft_token_id().get();
        let caller = self.blockchain().get_caller();
        self.send().direct(
            &caller,
            &nft_token_id,
            nft_nonce,
            &BigUint::from(NFT_AMOUNT),
            &[],
        );

        let owner = self.blockchain().get_owner_address();
        
        self.send()
            .direct(&owner, &TokenIdentifier::egld(), 0, &BigUint::from(SELLING_PRICE), &[]);

        Ok(nft_nonce)
    }

    fn require_token_issued(&self) -> SCResult<()> {
        require!(!self.nft_token_id().is_empty(), "Token not issued");
        Ok(())
    }

    fn require_local_roles_set(&self) -> SCResult<()> {
        let nft_token_id = self.nft_token_id().get();
        let roles = self.blockchain().get_esdt_local_roles(&nft_token_id);

        require!(
            roles.has_role(&EsdtLocalRole::NftCreate),
            "NFTCreate role not set"
        );

        Ok(())
    }

    // storage

    #[storage_mapper("nftTokenId")]
    fn nft_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("priceTag")]
    fn price_tag(&self, nft_nonce: u64) -> SingleValueMapper<PriceTag<Self::Api>>;
}
