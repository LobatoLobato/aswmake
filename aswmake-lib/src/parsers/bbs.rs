use std::collections::{HashSet, VecDeque};
use std::fmt::Display;
use std::{collections::HashMap};
use std::sync::LazyLock;

use indexmap::IndexMap;
use itertools::Itertools;
use lazy_regex::*;

use crate::error;
use crate::{tools, util};
use crate::path::{OptionalPath, Path};
use crate::parsers::loc::Loc;

#[derive(Debug)]
pub struct BBS {
    move_set: IndexMap<String, Move>
}

impl super::Parser for BBS  {
    fn ms_filter() -> &'static str {
        return "**/Chara/**/Data/**/BBS_*";
    }
}

impl BBS {
    pub fn parse(
        bbs_uexp_path: impl Path,
        out_dir: Option<impl Path>, 
        target_game: crate::TargetGame,
        loc: Option<&Loc>
    ) -> anyhow::Result<BBS> {
        use std::io::BufRead;
        let bbs_uexp_path = bbs_uexp_path.as_path();
        let bbs_file_name = bbs_uexp_path.file_name().ok_or(error::InvalidFilePath(bbs_uexp_path))?;
        let bbs_file_name = regex_replace!(r"(_\d+)?\.uexp", &bbs_file_name.to_string_lossy(), ".bbs").to_string();
        let out_dir = out_dir.as_path().or(bbs_uexp_path.parent()).unwrap();
        
        let bbs_path = out_dir.join(bbs_file_name);
        let bbscript_path = bbs_path.with_extension("bbscript");
        if bbs_uexp_path.exists() {
            tools::bbspack::extract(&bbs_uexp_path, &bbscript_path)?;
            tools::bbscript::parse(&bbscript_path, &bbs_path, target_game)?;
            let _ = std::fs::remove_file(bbscript_path);
        }
        
        let mut bbs = Self {
            move_set: IndexMap::new()
        };
        
        if let Ok(file) = std::fs::File::open(bbs_path) {
            let mut current_move: Option<Move> = None;
            
            for line in std::io::BufReader::new(file).lines() {
                let line = line?;
                
                if regex_is_match!(r"((addMove:)|(beginState:))", &line)  {
                    let id = Move::parse_id(&line);
                    current_move = bbs.move_set.swap_remove(id).or_else(|| Some(Move::new(&line, loc)));
                } else if let Some(mv) = current_move.take_if(|_| regex_is_match!(r"((endMove:)|(endState:))", &line)) {
                    bbs.move_set.insert(mv.id.clone(), mv);
                } else if let Some(mv) = &mut current_move { mv.try_parse(&line); }     
            }
            
            if bbs.move_set.len() == 0 {
                return Err(anyhow::Error::msg("Parsed move set is empty"));
            }
        }
        
        bbs.move_set.iter_mut().for_each(|(_, m)| m.finish());
        Ok(bbs)
    }
    
    pub fn render(&self) -> String {
        use itertools::Itertools; 
        let mut moves = self.move_set.values().filter(|mv| mv.has_impl()).sorted_by_key(|mv| {
            if mv.has_flag("T_MOVEMENT_UNI") { 0 }
            else if mv.has_flag("T_MOVEMENT") { 1 }
            else if mv.has_flags(&["T_NORMAL", "!CS_JUMPING"]) { 2 }
            else if mv.has_flags(&["T_NORMAL", "CS_JUMPING"]) { 3 } 
            else if mv.has_flag("T_SPECIAL") { 4 } 
            else if mv.has_flag("T_OVERDRIVE") { 5 } 
            else { 6 }
        }).collect::<Vec<&Move>>();
        
        fn put_after<'a>(mset: &'a mut Vec<&Move>, k1: &str, k2: &str) {
            if let Some(k1_index) = mset.iter().position(|m| m.id == k1) && 
               let Some(k2_index) = mset.iter().position(|m| m.id == k2) {
                let v = mset.remove(k1_index);
                mset.insert(k2_index, v);        
            }
        }
        put_after(&mut moves, "HomingJump", "NmlAtk5E");
        put_after(&mut moves, "NmlAtkThrow", "NmlAtk2E");
        put_after(&mut moves, "ThrowExe", "NmlAtkThrow");
        put_after(&mut moves, "NmlAtkAirThrow", "ThrowExe");
        put_after(&mut moves, "NmlAtk5F", "NmlAtkAir5E");
        put_after(&mut moves, "NmlAtk6F", "NmlAtk5F");
        
        let states = self.move_set.values().filter(|mv| !mv.has_impl())
            .sorted_by_key(|mv| (!mv.has_name()).then(|| mv.main_sprite()));
        
        let mut move_list = moves.iter().map(|a| *a).chain(states).filter(|m| { 
            !m.is_empty() && 
            !m.has_flag("DEBUG_EX") && 
            m.main_sprite().is_some()
        });
        
        format!("{{\n  {}\n}}", move_list.join(",\n  "))
    }
}

pub fn parse(
    bbs_path: impl Path, 
    out_dir: Option<impl Path>, 
    target_game: crate::TargetGame, 
    loc: Option<&Loc>
) -> anyhow::Result<BBS> {
    BBS::parse(bbs_path, out_dir, target_game, loc)
}


#[derive(Debug, PartialEq, Eq)]
pub enum PropValue {
    Const(String),
    Struct{kind: String, value: String},
    String(String),
    Call
}
impl PropValue {
    fn parse(value_str: &str) -> Vec<Self> {
        value_str.split(',').map(|v| {
            let v = v.trim();
            
            if v.starts_with('(') && v.ends_with(')') {
                PropValue::Const(v.trim_matches(['(', ')']).into())
            } else if v.ends_with(")") {
                let (kind, value) = v.split_once('(').unwrap();
                PropValue::Struct{kind: kind.into(), value: value.trim_end_matches(')').into() }
            } else if v.starts_with("s32'") {
                PropValue::String(v[3..].trim_matches(['\'', '\'']).into())
            } else if v.is_empty() { 
                PropValue::Call 
            } else { PropValue::Const(v.to_string()) }
        }).collect()
    }
    fn parse_i(value_str: &str, n: usize) -> Option<Self> {
        let mut values = PropValue::parse(value_str);
        if values.len() > n {Some(values.remove(n))} else {None}
    }
}


#[derive(Debug)]
struct Move {
    id: String,
    name: Option<String>,
    input: Vec<String>,
    sprites: Vec<String>,
    flags: HashSet<String>
}
impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}
impl Move {
    fn new(line: &str, loc: Option<&Loc>) -> Self {
        let id = Move::parse_id(line).to_string();
        let name = loc.and_then(|l| l.move_loc_get_owned(&id)).take_if(|_| !id.contains("NmlAtk"));
        Self {
            id: id.clone(),
            name: name,
            input: vec![],
            sprites: vec![],
            flags: HashSet::from_iter([id])
        }
    }
    
    pub fn has_impl(&self) -> bool {
        !self.input.is_empty()
    }
    pub fn is_empty(&self) -> bool {
        self.input.is_empty() && 
        self.flags.len() == 1 && 
        self.sprites.len() == 0
    }
    pub fn has_name(&self) -> bool {
        self.name.is_some() || self.flags.iter().any(|f| TOKEN_DICT.get(f.as_str()).is_some_and(|t| !t.force_name.is_empty()))
    }
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }
    pub fn has_flags(&self, flags: &[&str]) -> bool {
        flags.iter().fold(true, |acc, f| {
            if f.starts_with('!') { acc && !self.flags.contains(&f[1..])}
            else { acc && self.flags.contains(*f) }
        })
    }
    pub fn main_sprite(&self) -> Option<&String> {
        self.sprites.iter().find(|s| *s != "keep" && *s != "null")
    }
    
    fn parse_id(line: &str) -> &str {
        regex_captures!(r"s32'(.+?)'", line).unwrap().1
    }
    
    fn try_parse(&mut self, line: &str) {
        use PropValue as PV;
        type ParserFn = fn(&mut Move, &str);
        static PARSERS: LazyLock<HashMap<&str, ParserFn>> = LazyLock::new(|| HashMap::from_iter(vec![
            ("characterState", Move::parse_char_state as ParserFn),
            ("moveType",       Move::parse_move_type as ParserFn),
            ("moveInput",      Move::parse_input as ParserFn),
            ("addMoveFlag",    Move::parse_flag as ParserFn),
            ("sprite",         Move::parse_sprite as ParserFn),
            ("isFollowupMove", |s, value_str| if let Some(PV::Const(x)) = PropValue::parse_i(value_str, 0) && x == "1" {
                s.flags.insert("IS_FOLLOWUP".into());
            }),
        ]));
        
        if let Some((f, value_str)) = line.trim().split_once(":").and_then(|(i, v)| PARSERS.get(i).map(|f| (f, v))) {
            f(self, value_str);
        }
    }
    fn parse_char_state(&mut self, line: &str) {
        if let Some(PropValue::Const(char_state)) = PropValue::parse_i(line, 0) {
            self.flags.insert(format!("CS_{}", char_state));    
        } 
    }
    fn parse_move_type(&mut self, line: &str) {
        if let Some(PropValue::Const(move_type)) = PropValue::parse_i(line, 0) {
            self.flags.insert(format!("T_{}", move_type));
        }
    }
    fn parse_input(&mut self, line: &str) {
        if let Some(PropValue::Const(input)) = PropValue::parse_i(line, 0) {
            self.input.push(regex_remove!(r"INPUT_(PRESS_)?", &input).into())
        }
    } 
    fn parse_flag(&mut self, line: &str) {
        if let Some(PropValue::Const(flag)) = PropValue::parse_i(line, 0) {
            self.flags.insert(flag);    
        } 
    }
    fn parse_sprite(&mut self, line: &str) {
        if let Some(PropValue::String(sprite)) = PropValue::parse_i(line, 0) {
            self.sprites.push(sprite);    
        }
    }
    
    fn finish(&mut self) {
        let mut input: VecDeque<String> = VecDeque::new();
        for x in &self.input {
            if let Some(token) = TOKEN_DICT.get(x.as_str()) {
                if !token.to.is_empty() {
                    let to = token.to.to_string();
                    if token.prefix { input.push_front(to); } 
                    else { input.push_back(to); }
                }
            } else {
                input.push_back(x.clone());
            }
        }
        
        let mut additional_flags: Vec<String> = vec![];
        for x in &self.flags {
            if let Some(token) = TOKEN_DICT.get(x.as_str()) {
                let to = token.to.to_string();
                if token.prefix { input.push_front(to); } 
                else { input.push_back(to); }
                
                if !token.force_name.is_empty() { self.name = Some(token.force_name.to_string()); }
                
                additional_flags.extend(token.set_flags.iter().map(|v| String::from(*v))); 
            }
        }
        self.flags.extend(additional_flags);
        if self.has_flag("HomingJump") { 
            self.name = None; 
            self.flags.remove("T_MOVEMENT");
        }
        
        if input.len() == 1 && input[0].len() == 1 && !input[0].parse::<u8>().is_ok(){
            input.push_front("5".to_string())
        }
        
        self.input = input.into_iter().collect();
    }
    
    fn render(&self) -> String {
        let rendered_input = self.input.iter().join("");
        let name_display = self.name.as_ref().unwrap_or(&rendered_input);
        
        let mut fields = Vec::new();
        if !rendered_input.is_empty() {
            fields.push(format!("\"input\": \"{rendered_input}\""));
        }
        if let Some(sprite) = self.main_sprite() {
            fields.push(format!("\"sprite\": \"{sprite}\""));
        }
        
        format!("\"{name_display}( {} )\": {{ {} }}", self.id, fields.join(", "))   
    }
}

#[derive(Default)]
struct Token<'a> {
    to: &'a str,
    prefix: bool,
    force_name: &'a str,
    set_flags: &'a[&'a str]
}
static TOKEN_DICT: LazyLock<HashMap<&str, Token>> = LazyLock::new(|| HashMap::from_iter([
    // Input Tokens
    ("ANY_FORWARD", util::make!(Token { to: "6", prefix: true })),
    ("ANY_DOWN", util::make!(Token { to: "2", prefix: true })),
    ("ANY_DOWNDOWN", util::make!(Token { to: "22", prefix: true })),
    ("ANY_BACK", util::make!(Token { to: "4", prefix: true })),
    ("CLOSE_SLASH", util::make!(Token { to: "c.", prefix: true })),
    ("FAR_SLASH", util::make!(Token { to: "f.", prefix: true })),
    ("THROW", util::make!(Token { to: "6D || 4D", force_name: "Ground Throw" })),
    ("AIR_THROW", util::make!(Token { to: "j.6D || j.4D", force_name: "Air Throw" })),
    ("BOOLEAN_OR", util::make!(Token { to: " || " })),
    ("DASH", util::make!(Token { to: "🏃‍➡️" })),
    ("HOLD_DASH", util::make!(Token { to: "[66]" })),
    ("HOLD_P", util::make!(Token { to: "[P]" })),
    ("HOLD_K", util::make!(Token { to: "[K]" })),
    ("HOLD_S", util::make!(Token { to: "[S]" })),
    ("HOLD_H", util::make!(Token { to: "[H]" })),
    ("HOLD_D", util::make!(Token { to: "[D]" })),
    ("TAUNT", util::make!(Token { to: "Taunt" })),
    ("DISALLOW_UP", util::make!(Token { to: "" })),
    ("NOT_1", util::make!(Token { to: "" })),
    ("NOT_3", util::make!(Token { to: "" })),
    ("ANY_UP", util::make!(Token { to: "" })),
    
    // Char State Tokens
    ("CS_JUMPING", util::make!(Token { to: "j.", prefix: true })),
    
    // Other Properties
    ("IS_FOLLOWUP", util::make!(Token { to: ">", prefix: true })),
    
    // ID Tokens
    ("HomingJump", util::make!(Token{ to: "jump", set_flags: &["T_NORMAL"] })),
    ("CmnActJump", util::make!(Token{ to: "jump", set_flags: &["T_MOVEMENT_UNI"] })),
    ("CmnActAirFDash", util::make!(Token{ to: "j.66", set_flags: &["T_MOVEMENT_UNI"] })),
    ("CmnActAirBDash", util::make!(Token{ to: "j.44", set_flags: &["T_MOVEMENT_UNI"] })),
    ("NmlAtkThrow", util::make!(Token{ to: "6D || 4D", set_flags: &["T_NORMAL"] , force_name: "Ground Throw" })),
    ("NmlAtkAirThrow", util::make!(Token{ to: "j.6D || j.4D", set_flags: &["T_NORMAL"] , force_name: "Air Throw" })),
    ("ThrowExe", util::make!(Token{ to: "6D || 4D", set_flags: &["T_NORMAL"] , force_name: "Ground Throw" })),
    ("AirThrowExe", util::make!(Token{ to: "j.6D || j.4D", set_flags: &["T_NORMAL"] , force_name: "Air Throw" })),
    ("BKN_SpecialCancel_FDash", util::make!(Token{ to : "", force_name: "Parry Dash Cancel" })),
    ("DustFinish", util::make!(Token{ to: "X>X", set_flags: &["T_NORMAL"] })),
    ("WildAssault", util::make!(Token{ to: "236D", set_flags: &["T_SYSMEC"] }))
]));

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(bbs_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use crate::{TargetGame, path::NoPath, util::sha1_hash};

    use super::*;
    use std::{path::PathBuf, sync::Arc};
    use suitest::{before_all};
    use tempfile;
    
    #[derive(Debug)]
    struct Context {
        loc_inst: Loc,
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf,
        bbs_uexp_path: PathBuf,
        move_list_ref_path: PathBuf,
        out_dir: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("bbs"));
        let out_dir = tmp_fixtures_dir_path.join("output");
        let loc = Loc::parse(&tmp_fixtures_dir_path.join("REDGame.ref.uexp"), Some(&out_dir)).unwrap();
        
        (Arc::new(Context { 
            loc_inst: loc,
            _fixtures_dir: tmp_fixtures_dir,
            bbs_uexp_path: tmp_fixtures_dir_path.join("BBS_FAU.ref.uexp"),
            move_list_ref_path: tmp_fixtures_dir_path.join("move_list.ref.json"),
            fixtures_dir_path: tmp_fixtures_dir_path,
            out_dir: out_dir,
            
        }), ())
    }
    
    #[test]
    fn can_parse_to_out_dir_and_render_move_list(ctx: Arc<Context>) {
        let bbs = BBS::parse(&ctx.bbs_uexp_path, Some(&ctx.out_dir), TargetGame::GGST, Some(&ctx.loc_inst)).unwrap();
        let parsed_bbscript_path = &ctx.out_dir.join("BBS_FAU.bbscript");
        
        assert_eq!(bbs.render(), std::fs::read_to_string(&ctx.move_list_ref_path).unwrap());
        std::fs::exists(&parsed_bbscript_path).unwrap();
        assert_eq!(
            sha1_hash(&ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript")).ok(), 
            sha1_hash(parsed_bbscript_path).ok()
        )
    }
    
    #[test]
    fn can_parse_to_default_dir_and_render_move_list(ctx: Arc<Context>) {
        let bbs = BBS::parse(&ctx.bbs_uexp_path, NoPath, TargetGame::GGST, Some(&ctx.loc_inst)).unwrap();
        let parsed_bbscript_path = &&ctx.fixtures_dir_path.join("BBS_FAU.bbscript");
        
        assert_eq!(bbs.render(), std::fs::read_to_string(&ctx.move_list_ref_path).unwrap());
        std::fs::exists(&parsed_bbscript_path).unwrap();
        assert_eq!(
            sha1_hash(&ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript")).ok(), 
            sha1_hash(parsed_bbscript_path).ok()
        )
    }
}