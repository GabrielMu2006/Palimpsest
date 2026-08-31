use std::process::ExitCode;

fn main() -> ExitCode {
    match parse_args().and_then(|(entities, seconds)| {
        palimpsest_headless_runner::run(entities, seconds).map_err(|error| error.to_string())
    }) {
        Ok(metrics) => match serde_json::to_string(&metrics) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("failed to encode metrics: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn parse_args() -> Result<(u64, i64), String> {
    let mut entities = 100_u64;
    let mut seconds = 86_400_i64;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {argument}"))?;
        match argument.as_str() {
            "--entities" => {
                entities = value.parse().map_err(|_| "invalid --entities".to_owned())?;
            }
            "--seconds" => {
                seconds = value.parse().map_err(|_| "invalid --seconds".to_owned())?;
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok((entities, seconds))
}
