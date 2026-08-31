//! Private protocol helpers shared by the timing examples, not simulation API.
#![allow(dead_code)]
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args {
    pub persons: usize,
    pub seconds: i64,
    pub samples: usize,
    pub warmups: usize,
    pub seed: u64,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn parse<I, S>(iter: I, defaults: Args) -> Result<Args, ParseError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    parse_for(
        iter,
        defaults,
        &[
            "--persons",
            "--seconds",
            "--samples",
            "--warmups",
            "--seed",
            "--json",
        ],
    )
}

pub fn parse_for<I, S>(iter: I, defaults: Args, allowed: &[&str]) -> Result<Args, ParseError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let v: Vec<String> = iter.into_iter().map(Into::into).collect();
    let mut out = defaults;
    let mut seen = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < v.len() {
        let key = &v[i];
        if !allowed.contains(&key.as_str()) {
            return Err(ParseError(format!("unsupported flag: {key}")));
        }
        if key == "--json" {
            if !seen.insert(key.clone()) {
                return Err(ParseError("duplicate --json".into()));
            }
            out.json = true;
            i += 1;
            continue;
        }
        let field = match key.as_str() {
            "--persons" | "--seconds" | "--samples" | "--warmups" | "--seed" => key,
            _ => return Err(ParseError(format!("unknown or malformed flag: {key}"))),
        };
        if !seen.insert(field.clone()) {
            return Err(ParseError(format!("duplicate {field}")));
        }
        let value = v
            .get(i + 1)
            .ok_or_else(|| ParseError(format!("missing value for {field}")))?;
        if value.starts_with('-') {
            return Err(ParseError(format!("malformed value for {field}")));
        }
        match field.as_str() {
            "--persons" => {
                out.persons = value
                    .parse()
                    .map_err(|_| ParseError("persons must be an integer".into()))?;
            }
            "--seconds" => {
                out.seconds = value
                    .parse()
                    .map_err(|_| ParseError("seconds must be an integer".into()))?;
            }
            "--samples" => {
                out.samples = value
                    .parse()
                    .map_err(|_| ParseError("samples must be an integer".into()))?;
            }
            "--warmups" => {
                out.warmups = value
                    .parse()
                    .map_err(|_| ParseError("warmups must be an integer".into()))?;
            }
            "--seed" => {
                out.seed = value
                    .parse()
                    .map_err(|_| ParseError("seed must be an integer".into()))?;
            }
            _ => unreachable!(),
        }
        i += 2;
    }
    if out.samples == 0 {
        return Err(ParseError("samples must be positive".into()));
    }
    if out.seconds <= 0 {
        return Err(ParseError("seconds must be positive".into()));
    }
    Ok(out)
}

pub fn median<T: Copy + Ord>(values: &mut [T]) -> T {
    values.sort_unstable();
    values[values.len() / 2]
}
pub const fn defaults() -> Args {
    Args {
        persons: 100,
        seconds: 86_400,
        samples: 10,
        warmups: 2,
        seed: 42,
        json: false,
    }
}

pub fn configuration() -> serde_json::Value {
    serde_json::json!({
        "kernel": format!("{:?}", palimpsest_sim_core::KernelConfig::default()),
        "worldgen": format!("{:?}", palimpsest_sim_world::WorldGenConfig::default()),
        "hunger_raw_per_second": palimpsest_sim_ai::HUNGER_RATE_PER_SECOND,
        "fatigue_raw_per_second": palimpsest_sim_ai::FATIGUE_RATE_PER_SECOND,
        "event_digest": "fnv1a64-length-prefixed-json-v1",
        "median_rule": "sorted upper-middle index n/2"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_bad_protocol() {
        for a in [
            vec!["--wat"],
            vec!["--samples"],
            vec!["--samples", "0"],
            vec!["--samples", "1", "--samples", "2"],
        ] {
            assert!(parse(a, defaults()).is_err());
        }
    }
    #[test]
    fn upper_median_even() {
        assert_eq!(median(&mut [4, 1, 3, 2]), 3);
    }
    #[test]
    fn preserves_submicro_units() {
        let mut x = [1_u128, 2, 3];
        assert_eq!(median(&mut x), 2);
    }
}
