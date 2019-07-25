mod read;
mod write;

pub use read::ReadQueryExecutor;
pub use write::WriteQueryExecutor;

use crate::{
    query_builders::QueryBuilder,
    query_document::QueryDocument,
    response_ir::{Response, ResultIrBuilder},
    CoreError, CoreResult, OutputType, OutputTypeRef, QueryPair, QuerySchemaRef, ResultPair, ResultResolutionStrategy,
};
use connector::{
    filter::RecordFinder, Identifier, Query, QueryResult, ReadQuery, ReadQueryResult, SingleReadQueryResult,
    WriteQueryResult,
};
use prisma_models::{ModelRef, PrismaValue};
use std::borrow::Borrow;

/// Central query executor and main entry point into the query core.
pub struct QueryExecutor {
    read_executor: ReadQueryExecutor,
    write_executor: WriteQueryExecutor,
}

// Todo:
// - Partial execution semantics?
// - Do we need a clearer separation of queries coming from different query blocks? (e.g. 2 query { ... } in GQL)
// - ReadQueryResult should probably just be QueryResult
// - This is all temporary code until the larger query execution overhaul.
impl QueryExecutor {
    pub fn new(read_executor: ReadQueryExecutor, write_executor: WriteQueryExecutor) -> Self {
        QueryExecutor {
            read_executor,
            write_executor,
        }
    }

    /// Executes a query document, which involves parsing & validating the document,
    /// building queries and a query execution plan, and finally calling the connector APIs to
    /// resolve the queries and build reponses.
    pub fn execute(&self, query_doc: QueryDocument, query_schema: QuerySchemaRef) -> CoreResult<Vec<Response>> {
        // 1. Parse and validate query document (building)
        let queries = QueryBuilder::new(query_schema).build(query_doc)?;

        // 2. Build query plan
        // ...

        // 3. Execute query plan
        let results: Vec<ResultPair> = self.execute_queries(queries)?;

        // 4. Build IR response / Parse results into IR response
        // Ok(results
        //     .into_iter()
        //     .fold(ResultIrBuilder::new(), |builder, result| builder.add(result))
        //     .build())
        unimplemented!()
    }

    fn execute_queries(&self, queries: Vec<QueryPair>) -> CoreResult<Vec<ResultPair>> {
        queries.into_iter().map(|query| self.execute_query(query)).collect()
    }

    fn execute_query(&self, query: QueryPair) -> CoreResult<ResultPair> {
        let (query, strategy) = query;
        let model_opt = query.extract_model();

        let query_result = match query {
            Query::Read(read) => unimplemented!(),
            // self
            //     .read_executor
            //     .execute(read, &vec![])
            //     .map(|res| {
            //         let res = QueryResult::Read(res);
            //         match strategy {
            //             ResultResolutionStrategy::Serialize(ref typ) => ResultPair(),
            //             ResultResolutionStrategy::Dependent(dependent_pair) => match *dependent_pair {
            //                 (Query::Read(ReadQuery::RecordQuery(mut rq)), strategy) => {
            //                     // Inject required information into the query and execute
            //                     rq.record_finder = Some(Self::to_record_finder(&write_result, model)?);
            //                     self.read_executor.read_one(rq).map(|res| QueryResult::Read(res))
            //                 }
            //                 _ => unreachable!(), // Invariant for now
            //             },
            //         }}),
            Query::Write(write) => {
                // let model = write
                //     .extract_model()
                //     .expect("Expected write queries to have an associated model.");

                // self.write_executor.execute(write).map(|res| {
                //     match strategy {
                //     ResultResolutionStrategy::Serialize(ref typ) => ,
                //     ResultResolutionStrategy::Dependent(dependent_pair) => match *dependent_pair {
                //         (Query::Read(ReadQuery::RecordQuery(mut rq)), strategy) => {
                //             // Inject required information into the query and execute
                //             rq.record_finder = Some(Self::to_record_finder(&write_result, model)?);
                //             self.read_executor.read_one(rq).map(|res| QueryResult::Read(res))
                //         }
                //         _ => unreachable!(), // Invariant for now
                //     },
                // }})
                // })
                unimplemented!()
            }
        };

        self.resolve_result(query_result, strategy)
    }

    fn resolve_result(result: QueryResult, strategy: ResultResolutionStrategy, model: Option<ModelRef>) -> CoreResult<ResultPair> {
        // match strategy {
            //             ResultResolutionStrategy::Serialize(ref typ) => ResultPair(),
            //             ResultResolutionStrategy::Dependent(dependent_pair) => match *dependent_pair {
            //                 (Query::Read(ReadQuery::RecordQuery(mut rq)), strategy) => {
            //                     // Inject required information into the query and execute
            //                     rq.record_finder = Some(Self::to_record_finder(&write_result, model)?);
            //                     self.read_executor.read_one(rq).map(|res| QueryResult::Read(res))
            //                 }
            //                 _ => unreachable!(), // Invariant for now
            //             },
            //         }}),
        unimplemented!()
    }

    // /// Attempts to coerce the given write result into the provided output type.
    // fn coerce_result(result: WriteQueryResult, typ: &OutputTypeRef) -> CoreResult<QueryResult> {
    //     let value: PrismaValue = match result.identifier {
    //         Identifier::Id(id) => id.into(),
    //         Identifier::Count(c) => PrismaValue::from(c), // Requires object with one field that is usize / int / float, or single scalar type.
    //         Identifier::Record(r) => unimplemented!(),    // Requires object. Try coercing all fields of the object.
    //         Identifier::None => unimplemented!(),         // Null?
    //     };

    //     unimplemented!()
    // }

    // fn coerce_value_type(val: PrismaValue, typ: &OutputTypeRef) -> CoreResult<()> {
    //     match typ.borrow() {
    //         OutputType::Object(o) => unimplemented!(),
    //         OutputType::Opt(inner) => unimplemented!(),
    //         OutputType::Enum(e) => unimplemented!(),
    //         OutputType::List(inner) => unimplemented!(),
    //         OutputType::Scalar(s) => unimplemented!(),
    //     };

    //     unimplemented!()
    // }

    // fn coerce_scalar() -> CoreResult<()> {
    //     unimplemented!()
    // }

    /// Attempts to convert a write query result into a RecordFinder required for dependent queries.
    /// Assumes ID field is used as dependent field (which is true for now in the current execution model).
    fn to_record_finder(write_result: &WriteQueryResult, model: ModelRef) -> CoreResult<RecordFinder> {
        let id_field = model.fields().id();

        match &write_result.identifier {
            Identifier::Id(id) => Ok(RecordFinder::new(id_field, id)),
            Identifier::Record(r) => r
                .collect_id(&id_field.name)
                .map(|id_val| RecordFinder::new(id_field, id_val))
                .map_err(|err| err.into()),

            other => Err(CoreError::ConversionError(format!(
                "Impossible conversion of write query result {:?} to RecordFinder.",
                other
            ))),
        }
    }

    /// Returns db name used in the executor.
    pub fn db_name(&self) -> String {
        self.write_executor.db_name.clone()
    }
}
