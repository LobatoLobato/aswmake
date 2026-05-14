use regex::Regex;
use std::{collections::HashMap, fs, io::{BufRead}, path::{Path, PathBuf}, sync::LazyLock};
use encoding_rs_io::DecodeReaderBytesBuilder;
use crate::tools;

#[derive(Debug)]
pub struct Loc {
   move_loc_map: HashMap<String, String>
}

static RE_CMCR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"CMCR_(\w+)").unwrap());
static RE_SUB_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\^m((Atk)|(Btn)))|;").unwrap());
static RE_HOLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(.+\s*)\(Hold\)").unwrap());

impl Loc {
    fn utf16le_file_reader(path: &PathBuf) -> std::io::BufReader<encoding_rs_io::DecodeReaderBytes<fs::File, Vec<u8>>> {
        let loc_f = fs::File::open(path).unwrap();
        let decoder = DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding_rs::UTF_16LE))
                .build(loc_f);
        
        std::io::BufReader::new(decoder)
    }
    
    pub fn parse(loc_uexp_path: impl AsRef<Path>, out_dir: Option<impl AsRef<Path>>) -> Result<Self, Box<dyn std::error::Error>> {
        let loc_uexp_path = loc_uexp_path.as_ref().to_path_buf();
        let out_dir = out_dir.map(|d| d.as_ref().to_path_buf())
            .unwrap_or(loc_uexp_path.parent().unwrap().to_path_buf());
        let Some(loc_file_name) = loc_uexp_path.file_name() else {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, 
                format!("loc_uexp_path({}) is not a valid file path", loc_uexp_path.display())
            )));
        };
        let loc_file_path = out_dir.join(loc_file_name).with_extension("loc");
        
        
        std::fs::create_dir_all(&out_dir).unwrap();
        
        if loc_uexp_path.exists() {
            tools::bbspack::extract(&loc_uexp_path, &loc_file_path)?;
        }
        
        let mut loc_it = Loc::utf16le_file_reader(&loc_file_path).lines();
        let mut loc_inst = Self { move_loc_map: HashMap::new() };
        
        while let Some(Ok(line)) = loc_it.next() {
            let Some(caps) = RE_CMCR.captures(&line) else { continue; };
            let Some(Ok(next_line)) = loc_it.next() else { continue; };
            
            let mut move_cmcr = caps[1].to_string();
            if loc_inst.move_loc_map.contains_key(&move_cmcr) {
                move_cmcr = format!("{move_cmcr}_");
            }
            
            let mut move_name = RE_SUB_NAME.replace_all(next_line.trim(), "").into_owned();
            while move_name.contains("(Hold)") {
                move_name = RE_HOLD.replace_all(&move_name, |m: &regex::Captures| { format!("[{}]", m[1].trim()) }).into_owned();
            }

            loc_inst.move_loc_map.insert(move_cmcr, move_name.replace('"', ""));
        }
        
        Ok(loc_inst)
    }
    pub fn move_loc_get(&self, key: &str) -> Option<&String> {
        if self.move_loc_map.contains_key(key) {
            return self.move_loc_map.get(key);
        }
        // Try normalized search
        let key_norm = key.replace("_", "").to_lowercase();
        for (id, name) in &self.move_loc_map {
            if id.replace('_', "").to_lowercase().contains(&key_norm) {
                return Some(name)
            }
        }

        return None
    }
    pub fn bms_filter() -> &'static str {
        return "{}/Localization/INT/REDGame.uexp";
    }
}

pub fn parse(loc_uexp_path: impl AsRef<Path>, out_dir: Option<impl AsRef<Path>>) -> Result<Loc, Box<dyn std::error::Error>> {
    return Loc::parse(loc_uexp_path, out_dir);
}

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(loc_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use crate::util::sha1_hash;

    use super::*;
    use std::sync::Arc;
    use suitest::{before_all};
    use tempfile;
    
    #[derive(Debug)]
    struct Context {
        loc_inst: Loc,
        _fixtures_dir: tempfile::TempDir,
        loc_file_path: PathBuf,
        ref_loc_file_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("loc"));
        let out_dir = tmp_fixtures_dir_path.join("output");
        let loc = Loc::parse(&tmp_fixtures_dir_path.join("REDGame.uexp"), Some(&out_dir)).unwrap();
        
        (Arc::new(Context { 
            loc_inst: loc,
            _fixtures_dir: tmp_fixtures_dir,
            loc_file_path: out_dir.join("REDGame.loc"),
            ref_loc_file_path: tmp_fixtures_dir_path.join("REDGame.ref.loc")
        }), ())
    }
    
    #[test]
    fn can_get_localized_move_by_bbs_id(ctx: Arc<Context>) {
        let test_cases = [
            ("TCRAN_LandGuard", "Ground Block"),
            ("Kuebiko_A", "P Scarecrow"),
            ("EddieSummonB", "That's A Lot!"),
            ("EddieD", "Oppose"),
            ("USG_Special_01", "Ryuujin"),
            ("UNI_213_Atk", "Command Normal"),
            ("ASK_ASKSpecial3_S_", "Recover Mana"),
        ];
        for (id, expected_name) in test_cases {
            assert_eq!(ctx.loc_inst.move_loc_get(id), Some(&String::from(expected_name)));
        }
    }
    
    #[test]
    fn correctly_parses_locuexp_into_readable_format_and_into_json_dicts(ctx: Arc<Context>) {
        assert!(fs::exists(&ctx.loc_file_path).unwrap());
        assert!(fs::metadata(&ctx.loc_file_path).unwrap().len() > 0);
        
        assert_eq!(sha1_hash(&ctx.loc_file_path).ok(), sha1_hash(&ctx.ref_loc_file_path).ok());
    }
    
}
