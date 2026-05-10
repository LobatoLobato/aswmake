use const_format::concatcp;
use regex::Regex;
use std::{collections::HashMap, fs, io::{BufRead, Write}, path::PathBuf, sync::LazyLock};
use encoding_rs_io::DecodeReaderBytesBuilder;
use crate::tools;

pub struct Loc {
   move_loc_map: HashMap<String, String>
}

static RE_CMCR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"CMCR_(\w+)").unwrap());
static RE_SUB_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\^m((Atk)|(Btn)))|;").unwrap());
static RE_HOLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(.+\s*)\(Hold\)").unwrap());

impl Loc {
    const LOC_DIR: &str = "RED/Content/Localization/INT";
    const LOC_FILE: &str = concatcp!(Loc::LOC_DIR, "/REDGame.loc");
    const BMS_LOC_FILE: &str = concatcp!(Loc::LOC_DIR, "/REDGame.uexp");
    const MOVE_LOC_DICT_FILE: &str = concatcp!(Loc::LOC_DIR, "/moves.loc.json");
    
    
    fn utf16le_file_reader(path: &PathBuf) -> std::io::BufReader<encoding_rs_io::DecodeReaderBytes<fs::File, Vec<u8>>> {
        let loc_f = fs::File::open(path).unwrap();
        let decoder = DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding_rs::UTF_16LE))
                .build(loc_f);
        
        std::io::BufReader::new(decoder)
    }
    
    pub fn parse(bms_root_dir: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let bms_loc_file_path = bms_root_dir.join(Loc::BMS_LOC_FILE);
        let loc_file_path = bms_root_dir.join(Loc::LOC_FILE);
        let move_loc_dict_path = bms_root_dir.join(Loc::MOVE_LOC_DICT_FILE);
        
        if bms_loc_file_path.exists() {
            tools::BBSPACK(&["extract", bms_loc_file_path.to_str().unwrap(), loc_file_path.to_str().unwrap()])?;
        }
        
        let mut move_dict = fs::File::create(move_loc_dict_path).unwrap();
        let mut first_move = true;
        let mut loc_it = Loc::utf16le_file_reader(&loc_file_path).lines();
        let mut loc_inst = Self { move_loc_map: HashMap::new() };
        
        write!(move_dict, "{{\n")?;
        while let Some(Ok(line)) = loc_it.next() {
            let Some(caps) = RE_CMCR.captures(&line) else { continue; };
            let Some(Ok(next_line)) = loc_it.next() else { continue; };
            
            if !first_move { write!(move_dict, ",\n")?; }
            
            let mut move_cmcr = caps[1].to_string();
            if loc_inst.move_loc_map.contains_key(&move_cmcr) {
                move_cmcr = format!("{move_cmcr}_");
            }
            
            let mut move_name = RE_SUB_NAME.replace_all(next_line.trim(), "").into_owned();
            while move_name.contains("(Hold)") {
                move_name = RE_HOLD.replace_all(&move_name, |m: &regex::Captures| { format!("[{}]", m[1].trim()) }).into_owned();
            }
            move_name = move_name.replace('"', "");
            
            let json_move_name = serde_json::to_string(&move_name).unwrap();
            write!(move_dict, "  \"{}\": {}", move_cmcr, json_move_name)?;
            loc_inst.move_loc_map.insert(move_cmcr, json_move_name.replace('"', ""));
            
            first_move = false;
        }
        write!(move_dict, "\n}}")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use sequential_test::sequential;
    use serde_json;
    
    static FIXTURES_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
        let path_str = std::env::var("FIXTURES_DIR").expect("Missing FIXTURES_DIR env var");
        PathBuf::from(path_str)
    });
    static PAKCHUNK_PATH: LazyLock<PathBuf> = LazyLock::new(|| FIXTURES_DIR.join("pakchunk"));
    static SHARED_LOC: LazyLock<Loc> = LazyLock::new(|| Loc::parse(&PAKCHUNK_PATH).expect("Error in Loc::parse"));
        
    #[test]
    #[sequential]
    fn parses_without_panicking() {
        let _ = SHARED_LOC;
    }
    
    #[test]
    #[sequential]
    fn can_get_localized_move_by_bbs_id() {
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
            assert_eq!(SHARED_LOC.move_loc_get(id), Some(&String::from(expected_name)));
        }
    }
    
    #[test]
    #[sequential]
    fn correctly_parses_locuexp_into_readable_format_and_into_json_dicts() {
        let loc_fpath = PAKCHUNK_PATH.join(Loc::LOC_FILE);
        let move_dict_fpath = PAKCHUNK_PATH.join(Loc::MOVE_LOC_DICT_FILE);
        
        assert!(fs::exists(&loc_fpath).unwrap());
        assert!(fs::metadata(&loc_fpath).unwrap().len() > 0);
        assert!(fs::exists(&move_dict_fpath).unwrap());
        assert!(fs::metadata(&move_dict_fpath).unwrap().len() > 0);
        
        let move_dict_f = fs::read_to_string(&move_dict_fpath).unwrap();
        let move_dict_j: serde_json::Value = serde_json::from_str(&move_dict_f).expect("Invalid JSON for move dict");
        let move_map_j = serde_json::to_value(&SHARED_LOC.move_loc_map).unwrap();
        
        assert_eq!(move_dict_j.as_object().unwrap().len(), move_map_j.as_object().unwrap().len());
        assert_eq!(move_map_j, move_dict_j);
    }
}
