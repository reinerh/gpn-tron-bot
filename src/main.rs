use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;

static USERNAME : &str = "deki";
static PASSWORD : &str = "xxxxxxxx";


struct GameState {
    id: usize,
    map: Vec<Vec<Option<usize>>>,
    heads: HashMap<usize, (usize,usize)>,
}

impl GameState {
    fn new(id: usize, width: usize, height: usize) -> GameState {
        let line = vec![None; width];
        let map = vec![line; height];
        GameState { id, map, heads: HashMap::new() }
    }

    fn update_pos(&mut self, id: usize, x: usize, y: usize) {
        self.map[y][x] = Some(id);
        self.heads.insert(id, (x,y));
    }

    fn player_died(&mut self, id: usize) {
        for row in self.map.iter_mut() {
            for col in row {
                if Some(id) == *col {
                    *col = None;
                }
            }
        }
        self.heads.remove(&id);
    }

    fn neighboring_opponents(&self, pos: (usize, usize)) -> bool {
        let height = self.map.len();
        let width = self.map[0].len();
        for (id, opp_pos) in &self.heads {
            if *id == self.id {
                continue;
            }
            if ((opp_pos.0 - 1 + width) % width, opp_pos.1) == pos { return true; }
            if ((opp_pos.0 + 1) % width, opp_pos.1) == pos { return true; }
            if (opp_pos.0, (opp_pos.1 - 1 + height) % height) == pos { return true; }
            if (opp_pos.0, (opp_pos.1 + 1) % height) == pos { return true; }
        }
        false
    }

    fn reachable(&self, reached: &mut HashSet<(usize,usize)>, pos: (usize,usize), consider_neighbors: bool) -> usize {
        if self.map[pos.1][pos.0].is_some() || reached.contains(&pos) || (consider_neighbors && self.neighboring_opponents(pos)) {
            return reached.len();
        }
        let height = self.map.len();
        let width = self.map[0].len();
        reached.insert(pos);
        self.reachable(reached, ((pos.0+width-1)%width, pos.1), consider_neighbors);
        self.reachable(reached, ((pos.0+1)%width, pos.1), consider_neighbors);
        self.reachable(reached, (pos.0, (pos.1+height-1)%height), consider_neighbors);
        self.reachable(reached, (pos.0, (pos.1+1)%height), consider_neighbors);
        reached.len()
    }

    fn direction_with_max_distance(&self, consider_neighbors: bool) -> Option<String> {
        let (pos_x, pos_y) = *self.heads.get(&self.id).expect("current position not found");
        let height = self.map.len();
        let width = self.map[0].len();
        let mut distances = HashMap::new();
        distances.insert("up", self.reachable(&mut HashSet::new(), (pos_x, (pos_y + height - 1) % height), consider_neighbors));
        distances.insert("down", self.reachable(&mut HashSet::new(), (pos_x, (pos_y + 1) % height), consider_neighbors));
        distances.insert("left", self.reachable(&mut HashSet::new(), ((pos_x + width - 1) % width, pos_y), consider_neighbors));
        distances.insert("right", self.reachable(&mut HashSet::new(), ((pos_x + 1) % width, pos_y), consider_neighbors));
        distances.iter()
                 .filter(|x| *x.1 > 0)
                 .max_by_key(|x| x.1)
                 .map(|x| x.0.to_string())
    }

    fn next_move(&mut self) -> String {
        for consider_neighbors in [true, false] {
            if let Some(direction) = self.direction_with_max_distance(consider_neighbors) {
                return direction;
            }
        }
        "up".to_string()
    }
}

fn main() {
    let mut stream = TcpStream::connect("gpn-tron.duckdns.org:4000").expect("Couldn't connect to the server...");

    let mut game = GameState::new(0, 0, 0);

    loop {
        let mut buf = [0; 10240];
        let rx_len = stream.read(&mut buf).expect("read error");
        if rx_len == 0 {
            println!("end of data");
            return;
        }
        let buf = &buf[..rx_len];
        let received_str = match std::str::from_utf8(buf) {
            Ok(s) => s,
            Err(_) => {
                println!("received invalid utf8");
                continue;
            }
        };
        //println!("received {rx_len} bytes: '{received_str}'");
        let mut lines = received_str.split('\n').collect::<Vec<&str>>();
        let remainder = lines.pop().expect("no line received");
        if !remainder.is_empty() {
            panic!("received string contains an unterminated line: {remainder}");
        }
        for line in &lines {
            let args = line.split('|').collect::<Vec<&str>>();
            let cmd = args[0];
            //println!("received command: {cmd}");
            match cmd {
                "motd" => {
                    println!("MOTD: {}", args[1]);
                    let _ = stream.write(&format!("join|{USERNAME}|{PASSWORD}\n").into_bytes());
                    let _ = stream.flush();
                },
                "error" => {
                    println!("Error: {}", args[1]);
                    return;
                },
                "tick" => {
                    //println!("tick");
                    let next_move = game.next_move();
                    let _ = stream.write(&format!("move|{}\n", next_move).into_bytes());
                    let _ = stream.flush();
                    //println!("moving to: {next_move}");
                },
                "win" => { println!("YOU WON (wins: {}, losses: {})", args[1], args[2]); },
                "lose" => { println!("YOU LOSE (wins: {}, losses: {})", args[1], args[2]); },
                "game" => {
                    let width = args[1].parse::<usize>().expect(&format!("failed to parse width: {}", args[1]));
                    let height = args[2].parse::<usize>().expect(&format!("failed to parse height: {}", args[2]));
                    let id = args[3].parse::<usize>().expect(&format!("failed to parse id: {}", args[3]));
                    game = GameState::new(id, width, height);
                    println!("\nNEW GAME {width}x{height} (id: {id})!");
                },
                "pos" => {
                    let id = args[1].parse::<usize>().expect(&format!("failed to parse id: {}", args[1]));
                    let x = args[2].parse::<usize>().expect(&format!("failed to parse x: {}", args[2]));
                    let y = args[3].parse::<usize>().expect(&format!("failed to parse y: {}", args[3]));
                    //println!("received pos for {id}: {x}/{y}");
                    game.update_pos(id, x, y);
                }
                "message" => { println!("message from {}: {}", args[1], args[2]); },
                "die" => {
                    for player in args.iter().skip(1) {
                        game.player_died(player.parse::<usize>().expect("failed to parse player id"));
                    }
                    println!("players died: {}", args[1..].join(", "));
                },
                _ => println!("unsupported command: {cmd}"),
            }
        }
    }
}
