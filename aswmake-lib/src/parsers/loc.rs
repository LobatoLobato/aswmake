use lazy_regex::*;
use std::{collections::HashMap, fs, io::{BufRead}, path::{PathBuf}};
use encoding_rs_io::DecodeReaderBytesBuilder;
use crate::{path::{OptionalPath, Path}, tools};

use crate::error;

#[derive(Debug)]
pub struct Loc {
   move_loc_map: HashMap<String, String>
}

impl super::Parser for Loc {
    fn ms_filter() -> &'static str {
        return "{}/Localization/INT/REDGame.uexp";
    }
}

impl Loc {
    fn utf16le_file_reader(path: &PathBuf) -> std::io::BufReader<encoding_rs_io::DecodeReaderBytes<fs::File, Vec<u8>>> {
        let loc_f = fs::File::open(path).unwrap();
        let decoder = DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding_rs::UTF_16LE))
                .build(loc_f);
        
        std::io::BufReader::new(decoder)
    }
    
    pub fn parse(loc_uexp_path: impl Path, out_dir: Option<impl Path>) -> anyhow::Result<Self> {
        let loc_uexp_path = loc_uexp_path.as_path();
        let out_dir = out_dir.as_path().or(loc_uexp_path.parent()).unwrap();
        let loc_file_name = loc_uexp_path.file_name().ok_or(error::InvalidFilePath(loc_uexp_path))?;
        let loc_file_path = out_dir.join(loc_file_name).with_extension("loc");
        
        
        std::fs::create_dir_all(&out_dir).unwrap();
        
        if loc_uexp_path.exists() {
            tools::bbspack::extract(&loc_uexp_path, &loc_file_path)?;
        }
        
        let mut loc_it = Loc::utf16le_file_reader(&loc_file_path).lines();
        let mut loc_inst = Self { move_loc_map: HashMap::new() };
        
        while let Some(Ok(line)) = loc_it.next() {
            let Some(caps) = regex_captures!(r"CMCR_(\w+)", &line) else { continue; };
            let Some(Ok(next_line)) = loc_it.next() else { continue; };
            
            let mut move_cmcr = caps.1.to_string();
            if loc_inst.move_loc_map.contains_key(&move_cmcr) {
                move_cmcr = format!("{move_cmcr}_");
            }
            
            let mut move_name = regex_replace_all!(r"(\^m((Atk)|(Btn)))|;", next_line.trim(), "").into_owned();
            while move_name.contains("(Hold)") {
                move_name = regex_replace_all!(r"(.+\s*)\(Hold\)", &move_name, |_, m: &str| { format
                    !("[{}]", m.trim()) 
                }).into_owned();
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
    pub fn move_loc_get_owned(&self, key: &str) -> Option<String> {
        self.move_loc_get(key).map(String::to_owned)
    }
}

pub fn parse(loc_uexp_path: impl Path, out_dir: Option<impl Path>) -> anyhow::Result<Loc> {
    return Loc::parse(loc_uexp_path, out_dir);
}

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(loc_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use crate::{path::NoPath, util::sha1_hash};

    use super::*;
    use std::sync::Arc;
    use suitest::{before_all};
    use tempfile;
    
    #[derive(Debug)]
    struct Context {
        loc_inst: Loc,
        _fixtures_dir: tempfile::TempDir,
        loc_file_path_no_out_dir: PathBuf,
        loc_file_path_out_dir: PathBuf,
        ref_loc_file_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("loc"));
        let out_dir = tmp_fixtures_dir_path.join("output");
        
        let _loc_no_out_dir = Loc::parse(&tmp_fixtures_dir_path.join("REDGame.uexp"), NoPath).unwrap();
        let loc_out_dir = Loc::parse(&tmp_fixtures_dir_path.join("REDGame.uexp"), Some(&out_dir)).unwrap();        
        
        (Arc::new(Context { 
            loc_inst: loc_out_dir,
            _fixtures_dir: tmp_fixtures_dir,
            loc_file_path_no_out_dir: tmp_fixtures_dir_path.join("REDGame.loc"),
            loc_file_path_out_dir: out_dir.join("REDGame.loc"),
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
        assert!(fs::exists(&ctx.loc_file_path_no_out_dir).unwrap());
        assert!(fs::metadata(&ctx.loc_file_path_no_out_dir).unwrap().len() > 0);
        assert_eq!(sha1_hash(&ctx.loc_file_path_no_out_dir).ok(), sha1_hash(&ctx.ref_loc_file_path).ok());
        
        assert!(fs::exists(&ctx.loc_file_path_out_dir).unwrap());
        assert!(fs::metadata(&ctx.loc_file_path_out_dir).unwrap().len() > 0);
        assert_eq!(sha1_hash(&ctx.loc_file_path_out_dir).ok(), sha1_hash(&ctx.ref_loc_file_path).ok());
    }
    
}
