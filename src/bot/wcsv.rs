use csv::Writer;

use crate::bot::database::{
    CodeResult,
    UsernameResult,
};


pub fn create_csv_body_by_code(coderes: Vec<CodeResult>) -> String {
    let mut wtr = Writer::from_writer(vec![]);
    for code in coderes {
        for response in code.responses {
            wtr.serialize(response).unwrap();
        }
    }

    String::from_utf8(wtr.into_inner().unwrap()).unwrap()
}

pub fn create_csv_body_by_username(coderes: Vec<UsernameResult>) -> String {
    let mut wtr = Writer::from_writer(vec![]);
    for code in coderes {
        for response in code.responses {
            wtr.serialize(response).unwrap();
        }
    }

    String::from_utf8(wtr.into_inner().unwrap()).unwrap()
}

pub fn create_csv_body_aggregated_by_username(coderes: Vec<UsernameResult>) -> String {
    // Format: <username>,<code>,<code> ...
    let mut wtr = Writer::from_writer(vec![]);
    wtr.write_record(&["username", "speech_codes"]).unwrap();
    for code in coderes {
        let mut row = vec![code.username];
        for response in code.responses {
            row.push(response.speech_code);
        }
        wtr.serialize(row).unwrap();
    }

    String::from_utf8(wtr.into_inner().unwrap()).unwrap()
}
