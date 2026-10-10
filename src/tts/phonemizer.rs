//! eSpeak phonemization for Piper TTS.

use espeak_ng::{install_bundled_data, EspeakNg};
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::error::GxxError;
use crate::tts::model::PiperConfig;

static ESPEAK: OnceLock<Result<EspeakNg, GxxError>> = OnceLock::new();

/// Get the bundled data directory, installing if necessary.
fn get_data_dir() -> Result<PathBuf, GxxError> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| GxxError::msg("Could not determine local data directory"))?
        .join("gxx")
        .join("espeak-ng-data");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| GxxError::msg(format!("Failed to create data directory: {}", e)))?;

    // Install bundled data if not already present
    if !data_dir.join("en_dict").exists() {
        install_bundled_data(&data_dir).map_err(|e| {
            GxxError::msg(format!("Failed to install bundled espeak-ng data: {}", e))
        })?;
    }

    Ok(data_dir)
}

/// Initialize eSpeak NG for the given voice.
/// Returns an error if eSpeak is not available.
fn init_espeak(voice: &str) -> Result<&'static EspeakNg, GxxError> {
    let espeak_result = ESPEAK.get_or_init(|| {
        let data_dir = get_data_dir()?;
        EspeakNg::with_data_dir(voice, &data_dir)
            .map_err(|e| GxxError::msg(format!("Failed to initialize eSpeak NG: {}", e)))
            .map(|mut espeak| {
                espeak.set_voice(voice);
                espeak
            })
    });

    match espeak_result {
        Ok(espeak) => Ok(espeak),
        Err(e) => Err(GxxError::msg(e.to_string())),
    }
}

/// Split IPA phonemes into individual symbols that exist in the phoneme_id_map.
/// Uses longest-match algorithm to handle multi-character IPA symbols.
fn split_ipa_to_symbols(
    ipa: &str,
    phoneme_id_map: &std::collections::HashMap<String, Vec<u32>>,
) -> Result<Vec<String>, GxxError> {
    let mut symbols = Vec::new();
    let chars: Vec<char> = ipa.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Piper has an explicit space token. Preserve word boundaries.
        if chars[i].is_whitespace() {
            if phoneme_id_map.contains_key(" ") {
                symbols.push(" ".to_string());
            }
            i += 1;
            continue;
        }

        let mut matched = false;

        // IPA symbols in Piper's map may be multi-character sequences.
        // Use longest-match first, based on the actual model map.
        for len in (1..=4).rev() {
            if i + len > chars.len() {
                continue;
            }

            let candidate: String = chars[i..i + len].iter().collect();

            if phoneme_id_map.contains_key(&candidate) {
                symbols.push(candidate);
                i += len;
                matched = true;
                break;
            }
        }

        if !matched {
            let context_start = i.saturating_sub(3);
            let context_end = (i + 4).min(chars.len());
            let context: String = chars[context_start..context_end].iter().collect();

            return Err(GxxError::msg(format!(
                "eSpeak emitted IPA symbol not present in Piper phoneme_id_map: {:?} near {:?}",
                chars[i], context
            )));
        }
    }

    Ok(symbols)
}

/// Phonemize text using eSpeak NG.
/// Returns a vector of phoneme IDs matching the Piper model's phoneme_id_map.
///
/// Convert manually supplied IPA directly into Piper phoneme IDs.
///
/// Unlike `phonemize()`, this does NOT call eSpeak.
/// The caller controls the exact IPA sequence.
pub fn phonemize_manual(
    ipa: &str,
    phoneme_id_map: &std::collections::HashMap<String, Vec<u32>>,
) -> Result<Vec<u32>, GxxError> {
    let ipa = ipa.trim();

    if ipa.is_empty() {
        return Err(GxxError::msg("Manual phonemes cannot be empty"));
    }

    let symbols = split_ipa_symbols(ipa);

    let mut ids = Vec::new();

    // Piper normally expects BOS/EOS around the phoneme sequence.
    if let Some(bos) = phoneme_id_map.get("^") {
        ids.extend(bos);
    }

    for symbol in symbols {
        let mapped = phoneme_id_map.get(&symbol).ok_or_else(|| {
            GxxError::msg(format!(
                "Unknown manual phoneme '{}' in IPA '{}'",
                symbol, ipa
            ))
        })?;

        ids.extend(mapped);
    }

    if let Some(eos) = phoneme_id_map.get("$") {
        ids.extend(eos);
    }

    Ok(ids)
}

/// Split IPA using longest-match-first.
///
/// This is important for multi-character IPA symbols such as:
/// dʒ, tʃ, aɪ, eɪ, oʊ, etc.
fn split_ipa_symbols(ipa: &str) -> Vec<String> {
    // Piper Ryan does not expose dʒ/tʃ as single phoneme-map keys.
    // Expand them to the component symbols:
    //
    //   dʒ -> d + ʒ
    //   tʃ -> t + ʃ
    //
    // Other multi-character IPA symbols remain intact.

    const MULTI: &[&str] = &[
        "aɪ", "aʊ", "eɪ", "oʊ", "ɔɪ", "ɪə", "eə", "ʊə", "iː", "uː", "ɑː", "ɔː", "ɜː", "t̪", "d̪", "n̪",
    ];

    let chars: Vec<char> = ipa.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }

        // Model-level representation of affricates.
        if i + 1 < chars.len() {
            match (chars[i], chars[i + 1]) {
                ('d', 'ʒ') => {
                    out.push("d".to_string());
                    out.push("ʒ".to_string());
                    i += 2;
                    continue;
                }

                ('t', 'ʃ') => {
                    out.push("t".to_string());
                    out.push("ʃ".to_string());
                    i += 2;
                    continue;
                }

                _ => {}
            }
        }

        // Longest-match-first for the remaining multi-character symbols.
        let mut matched: Option<&str> = None;

        for candidate in MULTI {
            let candidate_chars: Vec<char> = candidate.chars().collect();

            if i + candidate_chars.len() <= chars.len()
                && chars[i..i + candidate_chars.len()] == candidate_chars[..]
            {
                matched = Some(candidate);
                break;
            }
        }

        if let Some(symbol) = matched {
            out.push(symbol.to_string());
            i += symbol.chars().count();
        } else {
            out.push(chars[i].to_string());
            i += 1;
        }
    }

    out
}

pub fn phonemize(text: &str, voice: &str, config: &PiperConfig) -> Result<Vec<u32>, GxxError> {
    eprintln!("TTS: requested voice = {:?}", voice);

    let espeak = init_espeak(voice)?;

    let phonemes = espeak
        .text_to_phonemes(text)
        .map_err(|e| GxxError::msg(format!("Phonemization failed: {}", e)))?;

    let symbols = split_ipa_to_symbols(&phonemes, &config.phoneme_id_map)?;

    eprintln!("TTS: eSpeak IPA = {:?}", phonemes);
    eprintln!("TTS: IPA symbols = {:?}", symbols);

    let bos = config
        .phoneme_id_map
        .get("^")
        .ok_or_else(|| GxxError::msg("Missing BOS '^' in phoneme_id_map"))?;

    let pad = config
        .phoneme_id_map
        .get("_")
        .ok_or_else(|| GxxError::msg("Missing PAD '_' in phoneme_id_map"))?;

    let eos = config
        .phoneme_id_map
        .get("$")
        .ok_or_else(|| GxxError::msg("Missing EOS '$' in phoneme_id_map"))?;

    let mut ids = Vec::new();

    // Piper 0.2.x input:
    // BOS + (phoneme + PAD) + ... + EOS
    ids.extend(bos.iter().copied());

    for symbol in symbols {
        let id_vec = config.phoneme_id_map.get(&symbol).ok_or_else(|| {
            GxxError::msg(format!(
                "Phoneme '{}' is missing from phoneme_id_map",
                symbol
            ))
        })?;

        ids.extend(id_vec.iter().copied());

        // Piper intersperses PAD between every phoneme.
        ids.extend(pad.iter().copied());
    }

    ids.extend(eos.iter().copied());

    eprintln!("TTS: phoneme IDs = {:?}", ids);

    Ok(ids)
}
#[allow(dead_code)]
pub fn phonemes_to_ids(
    phonemes: &[String],
    phoneme_id_map: &std::collections::HashMap<String, Vec<u32>>,
) -> Result<Vec<u32>, GxxError> {
    let mut ids = Vec::new();

    for phoneme in phonemes {
        if let Some(id_vec) = phoneme_id_map.get(phoneme) {
            ids.extend(id_vec.iter().copied());
        } else {
            // Try to handle unknown phonemes gracefully
            // Log a warning but continue with a fallback (space/unknown token)
            eprintln!(
                "Warning: phoneme '{}' not found in phoneme_id_map, skipping",
                phoneme
            );
            // Use space token (ID 3) as fallback
            if let Some(space_ids) = phoneme_id_map.get(" ") {
                ids.extend(space_ids.iter().copied());
            }
        }
    }

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_map(pairs: &[(&str, &[u32])]) -> HashMap<String, Vec<u32>> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_vec()))
            .collect()
    }

    #[test]
    fn test_phonemize_manual_simple() {
        let map = make_map(&[("^", &[1]), ("a", &[2]), ("b", &[3]), ("$", &[4])]);
        let ids = phonemize_manual("ab", &map).unwrap();
        assert_eq!(ids, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_phonemize_manual_empty() {
        let map = make_map(&[]);
        assert!(phonemize_manual("", &map).is_err());
        assert!(phonemize_manual("   ", &map).is_err());
    }

    #[test]
    fn test_phonemize_manual_whitespace() {
        let map = make_map(&[
            ("^", &[1]),
            ("a", &[2]),
            (" ", &[3]),
            ("b", &[4]),
            ("$", &[5]),
        ]);
        let ids = phonemize_manual("a b", &map).unwrap();
        // split_ipa_symbols skips whitespace, so BOS + a + b + EOS
        assert_eq!(ids, vec![1, 2, 4, 5]);
    }

    #[test]
    fn test_phonemize_manual_unknown_symbol() {
        let map = make_map(&[("^", &[1]), ("a", &[2]), ("$", &[3])]);
        // 'b' is not in the map
        assert!(phonemize_manual("ab", &map).is_err());
    }

    #[test]
    fn test_split_ipa_symbols_digraphs() {
        let result = split_ipa_symbols("aɪb");
        assert!(result.contains(&"aɪ".to_string()));
        assert!(result.contains(&"b".to_string()));
    }

    #[test]
    fn test_split_ipa_symbols_affricates() {
        let result = split_ipa_symbols("dʒ");
        assert!(result.contains(&"d".to_string()));
        assert!(result.contains(&"ʒ".to_string()));
    }

    #[test]
    fn test_split_ipa_symbols_tch() {
        let result = split_ipa_symbols("tʃ");
        assert!(result.contains(&"t".to_string()));
        assert!(result.contains(&"ʃ".to_string()));
    }
}
