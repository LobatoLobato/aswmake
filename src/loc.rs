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
            tools::BBSPACK(&["extract", bms_loc_file_path.to_str().unwrap(), Loc::LOC_FILE])?;
            fs::remove_file(&bms_loc_file_path)?;
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
            
            let move_cmcr = caps[1].to_string();
            let mut move_name = RE_SUB_NAME.replace_all(next_line.trim(), "").into_owned();
            while move_name.contains("(Hold)") {
                move_name = RE_HOLD.replace_all(&move_name, |m: &regex::Captures| {
                    format!("[{}]", m[1].trim())
                }).into_owned();
            }
            move_name = move_name.replace('"', "\\\"");
        
            write!(move_dict, "  \"{}\": \"{}\"", move_cmcr, move_name)?;
        
            loc_inst.move_loc_map.insert(move_cmcr, move_name);
            
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
        return "{}/Localization/INT/REDGame.uexp"
    }
}