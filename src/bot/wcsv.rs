use csv::Writer;

use crate::bot::database::{
    CodeResult
};


pub fn create_csv_body(coderes: Vec<CodeResult>) -> String {
    // Create a valid CSV body for all responses and output it as a String
    // Avoid error the trait bound `FullResponse: serde::Serialize` is not satisfied the following other types implement trait `serde::Serialize`
    let mut wtr = Writer::from_writer(vec![]);
    for code in coderes {
        for response in code.responses {
            wtr.serialize(response).unwrap();
        }
    }

    String::from_utf8(wtr.into_inner().unwrap()).unwrap()
}