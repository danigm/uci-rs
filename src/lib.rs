#[macro_use] extern crate log;

use std::process::{Child, Command, Stdio};

use std::io::Read;
use std::io::Write;

use std::fmt;
use std::thread;
use std::time::Duration;

use std::cell::RefCell;

mod error;
pub use error::{Result, EngineError};

pub struct Engine {
    engine: RefCell<Child>,

    movetime: u32,
    depth: Option<u32>,
}

const DEFAULT_TIME: u32 = 100;

impl Engine {
    /// Create a new [`Engine`] instance.
    ///
    /// # Arguments
    /// 
    /// * `path` - The path to the engine executable.
    ///
    /// # Panics
    ///
    /// * Panics if the engine couldn't be spawned (path is invalid, execution permission denied, etc.)
    ///
    /// [`Engine`]: struct.Engine.html
    pub fn new(path: &str) -> Result<Engine> {
        let cmd = Command::new(path)
                          .stdin(Stdio::piped())
                          .stdout(Stdio::piped())
                          .spawn()
                          .expect("Unable to run engine");

        let res = Engine {
            engine: RefCell::new(cmd),
            movetime: DEFAULT_TIME,
            depth: None,
        };

        res.read_line()?;
        res.command("uci")?;

        Ok(res)
    }

    /// Changes the amount of time the engine spends looking for a move
    ///
    /// # Arguments
    /// 
    /// * `new_movetime` - New timelimit in milliseconds
    pub fn movetime(mut self, new_movetime: u32) -> Engine {
        self.movetime = new_movetime;
        self
    }

    /// Changes the the engine depth when looking for a move
    ///
    /// # Arguments
    ///
    /// * `new_depth` - New depth, as an Option
    pub fn depth(mut self, new_depth: Option<u32>) -> Engine {
        self.depth = new_depth;
        self
    }

    /// Asks the engine to play the given moves from the initial position on it's internal board.
    /// 
    /// # Arguments
    ///
    /// * `moves` - A list of moves for the engine to play. Uses Coordinate notation
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = uci::Engine::new("stockfish").unwrap();
    /// let moves = vec!["e2e4".to_string(), "e7e5".to_string()];
    /// engine.make_moves(&moves).unwrap();
    /// ```
    pub fn make_moves(&self, moves: &[String]) -> Result<()> {
        self.write_fmt(format_args!("position startpos moves {}\n",
                                    moves.join(" ")))?;
        Ok(())
    }
    
    /// Asks the engine to use the position represented by the given FEN string
    /// 
    /// # Examples
    ///
    /// ```
    /// let engine = uci::Engine::new("stockfish").unwrap();
    /// engine.set_position("2k4R/8/3K4/8/8/8/8/8 b - - 0 1").unwrap();
    /// assert_eq!(engine.bestmove().unwrap(), "c8b7");
    /// ```
    pub fn set_position(&self, fen: &str) -> Result<()> {
        let moves: Vec<String> = vec![];
        self.make_moves_from_position(fen, &moves)
    }
    
    /// Asks the engine to use the position represented by the given FEN string
    /// and then play the given moves from that position
    pub fn make_moves_from_position(&self, fen: &str, moves: &Vec<String>) -> Result<()> {
        self.write_fmt(format_args!("position fen {} moves {}\n",
                                    fen, moves.join(" ")))?;
        Ok(())
    }
    
    fn do_move(&self) -> Result<()> {
        let movetime = self.movetime;
        if let Some(depth) = self.depth {
            self.write_fmt(format_args!("go movetime {movetime} depth {depth}\n"))?;
        } else {
            self.write_fmt(format_args!("go movetime {movetime}\n"))?;
        }

        Ok(())
    }

    /// Returns the best move in the current position according to the engine
    pub fn bestmove(&self) -> Result<String> {
        self.do_move()?;
        loop {
            let s = self.read_line()?;
            if s.starts_with("bestmove") {
                return Ok(s.split(" ").collect::<Vec<&str>>()[1].trim().to_string());
            }
        }
    }

    pub fn evaluation(&self) -> Result<i32> {
        self.do_move()?;
        let mut info = String::from("");
        loop {
            let s = self.read_line()?;
            if s.starts_with("info") {
                info = s.clone();
            }
            if s.starts_with("bestmove") {
                break;
            }
        }

        // info depth 25 seldepth 34 multipv 1 score cp -1933 nodes 18521596 nps 853018 hashfull 990 tbhits 0 time 21713 pv d2d3
        let parts = info.split(' ').collect::<Vec<&str>>();
        let cp_index = match parts.iter().enumerate().find(|(_i, v)| *v == &"cp") {
            Some((i, _v)) => i + 1,
            None => return Err(EngineError::NotFound)
        };

        match parts[cp_index].parse::<i32>() {
            Err(_e) => Err(EngineError::NotFound),
            Ok(n) => Ok(n),
        }
    }
    
    /// Sets an engine specific option to the given value
    ///
    /// # Arguments
    ///
    /// * `name`  - Name of the option
    /// * `value` - New value for the option
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = uci::Engine::new("stockfish").unwrap();
    /// engine.set_option("Skill Level", "5").unwrap();
    /// ```
    pub fn set_option(&self, name: &str, value: &str) -> Result<()> {
        self.write_fmt(format_args!("setoption name {} value {}\n",
                                    name, value))?;
        let error_msg =  self.read_left_output()?;
        
        if error_msg.trim().is_empty() {
            Ok(())
        } else {
            Err(EngineError::UnknownOption(name.to_string()))
        }
    }
    
    /// Sends a command to the engine and returns the output
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = uci::Engine::new("stockfish").unwrap();
    /// let analysis = engine.command("go depth 10").unwrap();
    /// println!("{}", analysis);
    /// ```
    pub fn command(&self, cmd: &str) -> Result<String> {
        self.write_fmt(format_args!("{}\n", cmd.trim()))?;
        thread::sleep(Duration::from_millis(100));
        self.read_left_output()
    }

    fn read_left_output(&self) -> Result<String> {
        let mut s: Vec<String> = vec![];

        self.write_fmt(format_args!("isready\n"))?;
        loop {
            let next_line = self.read_line()?;
            match next_line.trim() {
                "readyok" => return Ok(s.join("\n")),
                other     => s.push(other.to_string())
            }
        }
    }

    fn write_fmt(&self, args: fmt::Arguments) -> Result<()> {
        info!("Command: {:?}", fmt::format(args));
        self.engine.borrow_mut().stdin.as_mut().unwrap().write_fmt(args)?;
        Ok(())
    }

    fn read_line(&self) -> Result<String> {
        let mut s = String::new();
        let mut buf: Vec<u8> = vec![0];

        loop {
            self.engine.borrow_mut().stdout.as_mut().unwrap().read(&mut buf)?;
            s.push(buf[0] as char);
            if buf[0] == '\n' as u8 {
                break
            }
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let engine = Engine::new("./stockfish").unwrap().movetime(200);
        engine.set_option("Skill Level", "15").unwrap();
        let t = engine.bestmove().unwrap();
    }

    #[test]
    fn test_depth() {
        let engine = Engine::new("./stockfish").unwrap().movetime(50).depth(Some(1));
        engine.set_option("Skill Level", "0").unwrap();
        engine.set_position("6b1/8/1k5P/8/1P3B2/5pp1/8/4K3 b - - 0 1").unwrap();

        let t = engine.bestmove().unwrap();

        assert_eq!("f3f2", t);
    }

    #[test]
    fn test_evaluation() {
        let engine = Engine::new("./stockfish").unwrap().movetime(5000).depth(Some(40));
        engine.set_position("6b1/8/1k5P/8/1P3B2/5pp1/8/4K3 b - - 0 1").unwrap();
        let t = engine.evaluation().unwrap();
        assert!(t > 6000);
    }
}
