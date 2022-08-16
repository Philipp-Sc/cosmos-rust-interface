# cosmos-rust-interface

- Interface with cosmos chains, used by [cosmos-rust-bot](https://github.com/Philipp-Sc/cosmos-rust-bot).

## developer notes

- wraps return type within common structure / ResponseResult. 
 `Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(res)))`
 
- provides post-processing given a set of ResponseResults, returning a list of entries.

## Dependencies

- <a href="https://github.com/Philipp-Sc/cosmos-rust-package">Philipp-Sc/cosmos-rust-package</a>
