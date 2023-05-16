#[cfg(test)]
pub mod test {
    use borsh::{BorshDeserialize, BorshSerialize};
    use serde::{Deserialize, Serialize};
    use sov_app_template::{Batch, SequencerOutcome};
    use sov_modules_api::{
        default_context::DefaultContext, default_signature::private_key::DefaultPrivateKey, Address,
    };
    use sov_state::ProverStorage;
    use sovereign_sdk::{
        core::mocks::MockZkvm, da::BlobTransactionTrait, stf::StateTransitionFunction,
    };

    use crate::{
        app::{create_demo_config, create_new_demo, C, LOCKED_AMOUNT, SEQUENCER_DA_ADDRESS},
        data_generation::{simulate_da, QueryGenerator},
        helpers::query_and_deserialize,
        runtime::Runtime,
    };

    #[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
    pub struct TestBlob {
        address: Address,
        data: Vec<u8>,
    }

    impl BlobTransactionTrait for TestBlob {
        type Data = std::io::Cursor<Vec<u8>>;

        type Address = Address;

        fn sender(&self) -> Self::Address {
            self.address.clone()
        }

        fn data(&self) -> Self::Data {
            std::io::Cursor::new(self.data.clone())
        }
    }

    impl TestBlob {
        pub fn new(batch: Batch, address: &[u8]) -> Self {
            Self {
                address: TryInto::<Address>::try_into(address).unwrap(),
                data: batch.try_to_vec().unwrap(),
            }
        }
    }

    #[test]
    fn test_demo_values_in_db() {
        let path = schemadb::temppath::TempPath::new();
        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT + 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );
        {
            let mut demo = create_new_demo(&path);

            StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
            StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

            let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

            let apply_blob_outcome = StateTransitionFunction::<MockZkvm>::apply_blob(
                &mut demo,
                TestBlob::new(Batch { txs }, &SEQUENCER_DA_ADDRESS),
                None,
            )
            .inner;
            assert!(
                matches!(apply_blob_outcome, SequencerOutcome::Rewarded,),
                "Sequencer execution should have succeeded but failed "
            );
            StateTransitionFunction::<MockZkvm>::end_slot(&mut demo);
        }

        // Generate a new storage instance after dumping data to the db.
        {
            let runtime = &mut Runtime::<DefaultContext>::new();
            let storage = ProverStorage::with_path(&path).unwrap();

            let resp = query_and_deserialize::<election::query::GetResultResponse>(
                runtime,
                QueryGenerator::generate_query_election_message(),
                storage.clone(),
            );

            assert_eq!(
                resp,
                election::query::GetResultResponse::Result(Some(election::Candidate {
                    name: "candidate_2".to_owned(),
                    count: 3
                }))
            );

            let resp = query_and_deserialize::<value_setter::query::Response>(
                runtime,
                QueryGenerator::generate_query_value_setter_message(),
                storage,
            );

            assert_eq!(resp, value_setter::query::Response { value: Some(33) });
        }
    }

    #[test]
    fn test_demo_values_in_cache() {
        let path = schemadb::temppath::TempPath::new();
        let mut demo = create_new_demo(&path);

        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT + 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );

        StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
        StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

        let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

        let apply_blob_outcome = StateTransitionFunction::<MockZkvm>::apply_blob(
            &mut demo,
            TestBlob::new(Batch { txs }, &SEQUENCER_DA_ADDRESS),
            None,
        )
        .inner;
        assert!(
            matches!(apply_blob_outcome, SequencerOutcome::Rewarded,),
            "Sequencer execution should have succeeded but failed "
        );
        StateTransitionFunction::<MockZkvm>::end_slot(&mut demo);

        let runtime = &mut Runtime::<DefaultContext>::new();
        let resp = query_and_deserialize::<election::query::GetResultResponse>(
            runtime,
            QueryGenerator::generate_query_election_message(),
            demo.current_storage.clone(),
        );

        assert_eq!(
            resp,
            election::query::GetResultResponse::Result(Some(election::Candidate {
                name: "candidate_2".to_owned(),
                count: 3
            }))
        );

        let resp = query_and_deserialize::<value_setter::query::Response>(
            runtime,
            QueryGenerator::generate_query_value_setter_message(),
            demo.current_storage.clone(),
        );

        assert_eq!(resp, value_setter::query::Response { value: Some(33) });
    }

    #[test]
    fn test_demo_values_not_in_db() {
        let path = schemadb::temppath::TempPath::new();

        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT + 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );
        {
            let mut demo = create_new_demo(&path);

            StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
            StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

            let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

            let apply_blob_outcome = StateTransitionFunction::<MockZkvm>::apply_blob(
                &mut demo,
                TestBlob::new(Batch { txs }, &SEQUENCER_DA_ADDRESS),
                None,
            )
            .inner;
            assert!(
                matches!(apply_blob_outcome, SequencerOutcome::Rewarded,),
                "Sequencer execution should have succeeded but failed "
            );
        }

        // Generate a new storage instance, value are missing because we didn't call `end_slot()`;
        {
            let runtime = &mut Runtime::<C>::new();
            let storage = ProverStorage::with_path(&path).unwrap();
            let resp = query_and_deserialize::<election::query::GetResultResponse>(
                runtime,
                QueryGenerator::generate_query_election_message(),
                storage.clone(),
            );

            assert_eq!(
                resp,
                election::query::GetResultResponse::Err("Election is not frozen".to_owned())
            );

            let resp = query_and_deserialize::<value_setter::query::Response>(
                runtime,
                QueryGenerator::generate_query_value_setter_message(),
                storage,
            );

            assert_eq!(resp, value_setter::query::Response { value: None });
        }
    }

    #[test]
    fn test_sequencer_insufficient_funds() {
        let path = schemadb::temppath::TempPath::new();

        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT - 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );

        let mut demo = create_new_demo(&path);

        StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
        StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

        let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

        let apply_blob_result = StateTransitionFunction::<MockZkvm>::apply_blob(
            &mut demo,
            TestBlob::new(Batch { txs }, &SEQUENCER_DA_ADDRESS),
            None,
        )
        .inner;
        assert!(
            matches!(apply_blob_result, SequencerOutcome::Ignored),
            "Batch should have been skipped due to insufficient funds"
        );
    }
}