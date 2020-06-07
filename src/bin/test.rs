use anyhow::Error;
use nmea::Nmea;
use std::io::BufRead;

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    let file = std::fs::File::open("example.txt")?;
    let reader = std::io::BufReader::new(file);

    let mut nmea = Nmea::new();
    for line in reader.lines() {
        let parsed = nmea.parse(&line?);
        println!("{:?}", parsed);

        match parsed {
            Ok(nmea::SentenceType::TXT) => {
                println!("txt: {:?}", nmea.last_txt());
            }
            _ => (),
        }
    }
    // println!("{:#?}", nmea);

    Ok(())
}
