use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Write},
    ops::{Deref, DerefMut},
    time::Duration,
};

use ircie::{
    format::{Color, Msg},
    system::IntoResponse,
    system_params::{AnyArguments, Res, ResMut},
    Irc,
};
use itertools::Itertools;
use rand::{
    rngs::StdRng,
    seq::{IteratorRandom, SliceRandom},
    Rng, SeedableRng,
};
use serde::{Deserialize, Serialize};

const SAVE_LOC: &'static str = "hall_of_fame.yaml";

const PRESENTATIONS: &'static [&'static str] = &[
    "A faggot coming straight from the underground!",
    "HARDCHATTER from the pong dynasty!",
    "Tonight's most dangerous tranny!",
    "Who the fuck invited this nigga? ! ? ! ? ! ? ! ?",
];

const BODYPARTS: &'static [&'static str] = &[
    "head",
    "groin",
    "left arm",
    "right arm",
    "left leg",
    "right leg",
    "soul",
    "left eye",
    "right eye",
    "mouth",
    "nose",
    "chin",
    "left ear",
    "right ear",
    "nick",
    "torso",
    "stomach",
    "ribs",
    "left foot",
    "right foot",
    "ass",
    "small dick",
    "deep pussy",
    "hair",
    "purse",
    "sunglasses",
    "teeth",
    "anus",
];

const ACTIONS: &'static [&'static str] = &[
    "punches",
    "kicks",
    "bites",
    "scratches",
    "pinches",
    "high kicks",
    "low kicks",
    "throws a left hook at",
    "throws a right hook at",
    "does a flying heel kick at",
    "slaps",
    "headbutts",
    "hits",
    "hammerfists",
    "front kicks",
    "stomps",
    "packets",
    "sues",
    "hacks",
    "doxes",
    "haunts",
    "curses with chaos magick",
    "violently hisses at",
    "cums on",
    "lumps out",
    "humps",
];

const COLORS: &'static [&'static Color] = &[
    &Color::Black,
    &Color::Blue,
    &Color::Brown,
    &Color::Cyan,
    &Color::Gray,
    &Color::Green,
    &Color::LightBlue,
    &Color::LightGray,
    &Color::LightGreen,
    &Color::Magenta,
    &Color::Orange,
    &Color::Purple,
    &Color::Red,
    &Color::Teal,
];

struct Fighter {
    nick: String,
    health: f32,
    color: Color,
    team_idx: usize,
}

#[derive(Default, Serialize, Deserialize)]
struct HallOfFame(HashMap<String, isize>);

impl Deref for HallOfFame {
    type Target = HashMap<String, isize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HallOfFame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl HallOfFame {
    pub fn add_winner(&mut self, nick: &str) {
        let winner = match self.entry(nick.to_owned()) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => v.insert(0),
        };
        *winner += 3;
    }
    pub fn add_fucking_looser(&mut self, nick: &str) {
        let fucking_looser = match self.entry(nick.to_owned()) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => v.insert(0),
        };

        *fucking_looser -= 1;
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        let Ok(mut file) = File::open(path) else {
            return Ok(Self::default());
        };

        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(serde_yaml::from_str(&content).unwrap())
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let content_str = serde_yaml::to_string(self).unwrap();
        let mut content = content_str.as_bytes();

        let mut file = File::create(path)?;

        file.write_all(&mut content)?;
        Ok(())
    }
}

impl Fighter {
    pub fn new(nick: &str, color: Color, team_idx: usize) -> Self {
        Self {
            nick: nick.to_owned(),
            health: 100.,
            color,
            team_idx,
        }
    }
}

impl Default for Fighter {
    fn default() -> Self {
        Self {
            nick: Default::default(),
            health: 100.,
            color: Color::Teal,
            team_idx: 0,
        }
    }
}

#[derive(Default, PartialEq, Eq)]
enum FightKind {
    #[default]
    Duel,
    FreeForAll,
    TeamBattle,
}

#[derive(Default, PartialEq, Eq)]
enum FightStatus {
    Happening,
    #[default]
    Idle,
}

#[derive(Default)]
struct Fight {
    status: FightStatus,
    kind: FightKind,
    fighters: Vec<Fighter>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let mut irc = Irc::from_config("irc_config.yaml").await?;

    irc.add_resource(Fight::default())
        .await
        .add_resource(StdRng::from_entropy())
        .await
        .add_resource(HallOfFame::load(SAVE_LOC).unwrap())
        .await
        .add_interval_task(Duration::from_secs(1), fight)
        .await
        .add_system("f", new_fight)
        .await
        .add_system("hof", show_hall_of_fame)
        .await;

    irc.run().await?;
    Ok(())
}

fn fight(
    mut fight: ResMut<Fight>,
    mut rng: ResMut<StdRng>,
    mut hall_of_fame: ResMut<HallOfFame>,
) -> impl IntoResponse {
    if fight.status == FightStatus::Idle {
        std::thread::sleep(Duration::from_millis(50));
        return None;
    }

    let teams_remaining: Vec<_> = fight
        .fighters
        .iter()
        .map(|f| f.team_idx)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if teams_remaining.len() == 1 {
        let team_idx = *teams_remaining.iter().next().unwrap();
        fight.status = FightStatus::Idle;

        let winners = fight
            .fighters
            .iter()
            .filter(|f| f.team_idx == team_idx)
            .collect::<Vec<_>>();

        let mut lines = vec![];

        if fight.fighters.len() == 1 {
            lines.push(
                Msg::new()
                    .color(Color::Yellow)
                    .text("We have a winner! -> ")
                    .color(winners[0].color)
                    .text(&winners[0].nick)
                    .color(Color::Yellow)
                    .text(" <- !"),
            );
            hall_of_fame.add_winner(&winners[0].nick);
        } else {
            lines.push(Msg::new().color(Color::Yellow).text("We have winners!"));
            for w in &winners {
                lines.push(
                    Msg::new()
                        .color(Color::Yellow)
                        .text("-> ")
                        .color(w.color)
                        .text(&w.nick)
                        .color(Color::Yellow)
                        .text(" <-"),
                );

                hall_of_fame.add_winner(&w.nick);
            }
        }

        for w in &winners {
            lines.push(Msg::new().text("!beer ").text(&w.nick));
        }

        fight.fighters = vec![];
        hall_of_fame.save(SAVE_LOC).unwrap();

        return Some((false, lines));
    }

    let mut lines = vec![];

    let body_part = BODYPARTS.choose(&mut *rng).unwrap();
    let action = ACTIONS.choose(&mut *rng).unwrap();
    let damage = rng.gen::<f32>() * 80.;

    let victim_idx = rng.gen_range(0..fight.fighters.len());
    let fucking_victim = fight.fighters.get_mut(victim_idx).unwrap();
    fucking_victim.health -= damage;

    let fucking_victim = fight.fighters.get(victim_idx).unwrap();

    let attacker = if fight
        .fighters
        .iter()
        .filter(|f| f.team_idx == fucking_victim.team_idx && f.nick != fucking_victim.nick)
        .count()
        != 0
        && rng.gen_bool(1. / 4.)
    {
        let attacker = fight
            .fighters
            .iter()
            .filter(|f| f.team_idx == fucking_victim.team_idx && f.nick != fucking_victim.nick)
            .choose(&mut *rng)
            .unwrap();

        lines.push(
            Msg::new()
                .color(Color::Yellow)
                .text("Oh no! ")
                .color(attacker.color)
                .text(&attacker.nick)
                .color(Color::Yellow)
                .text(&format!(" is fucking retarded and is attacking his mate!")),
        );
        attacker
    } else {
        fight
            .fighters
            .iter()
            .filter(|f| f.team_idx != fucking_victim.team_idx)
            .choose(&mut *rng)
            .unwrap()
    };

    lines.push(
        Msg::new()
            .color(attacker.color)
            .text(&attacker.nick)
            .reset()
            .text(&format!(" {} ", action))
            .color(fucking_victim.color)
            .text(&fucking_victim.nick)
            .reset()
            .text(&format!("'s {}!", body_part)),
    );

    if fucking_victim.health <= 0. {
        lines.push(
            Msg::new()
                .color(fucking_victim.color)
                .text(&fucking_victim.nick)
                .color(Color::Yellow)
                .text(" is lying dead!"),
        );
        hall_of_fame.add_fucking_looser(&fucking_victim.nick);
        fight.fighters.remove(victim_idx);
    }

    Some((false, lines))
}

fn new_fight(
    arguments: AnyArguments,
    mut fight: ResMut<Fight>,
    mut rng: ResMut<StdRng>,
) -> impl IntoResponse {
    if fight.status == FightStatus::Happening {
        return Err("Shut up and watch the show".to_owned());
    }

    if arguments.len() < 2 {
        return Err("nigga's supposed to fight alone?".to_owned());
    }

    fight.kind = if arguments.len() == 2 {
        FightKind::Duel
    } else if arguments.contains(&"vs") {
        FightKind::TeamBattle
    } else {
        FightKind::FreeForAll
    };

    let mut colors = COLORS.iter().map(|c| *c.clone()).collect::<Vec<_>>();
    colors.shuffle(&mut *rng);

    match fight.kind {
        FightKind::Duel => fight.fighters.append(&mut vec![
            Fighter::new(arguments[0], colors.pop().unwrap(), 0),
            Fighter::new(arguments[1], colors.pop().unwrap(), 1),
        ]),
        FightKind::FreeForAll => {
            for (idx, f) in arguments.iter().enumerate() {
                let color = colors.pop().unwrap();
                fight.fighters.push(Fighter::new(f, color, idx));

                if colors.len() == 0 {
                    colors = COLORS.iter().map(|c| *c.clone()).collect::<Vec<_>>();
                    colors.shuffle(&mut *rng);
                }
            }
        }
        FightKind::TeamBattle => {
            let mut team_idx = 0;
            let mut team_color = colors.pop().unwrap();
            for f in arguments.iter() {
                if f == &"vs" {
                    team_idx += 1;
                    team_color = colors.pop().unwrap();
                    continue;
                }
                fight.fighters.push(Fighter::new(f, team_color, team_idx));
            }
        }
    }

    fight.status = FightStatus::Happening;

    let mut init_msg = vec![Msg::new()
        .color(Color::Yellow)
        .text("THE FIGHT IS ABOUT TO BEGIN! TAKE YOUR BETS!")];

    match fight.kind {
        FightKind::Duel => init_msg.push(
            Msg::new()
                .color(Color::Yellow)
                .text("Tonight's fight is a duel!"),
        ),
        FightKind::FreeForAll => init_msg.push(
            Msg::new()
                .color(Color::Yellow)
                .text("Tonight's fight is a free for all!"),
        ),
        FightKind::TeamBattle => init_msg.push(
            Msg::new()
                .color(Color::Yellow)
                .text("Tonight's fight is a team battle!"),
        ),
    }

    init_msg.push(
        Msg::new()
            .color(Color::Yellow)
            .text("Everyone please welcome our contenders:"),
    );

    for f in &fight.fighters {
        init_msg.push(
            Msg::new()
                .color(f.color)
                .text(&f.nick)
                .color(Color::Yellow)
                .text("! ")
                .text(PRESENTATIONS.choose(&mut *rng).unwrap()),
        )
    }

    init_msg.push(Msg::new().color(Color::Yellow).text("TO THE DEATH!"));

    Ok((false, init_msg))
}

fn show_hall_of_fame(hall_of_fame: Res<HallOfFame>) -> impl IntoResponse {
    let sorted_hof = hall_of_fame
        .iter()
        .sorted_by_key(|k| k.1)
        .rev()
        .collect::<Vec<_>>();
    let mut lines = vec![Msg::new()
        .color(Color::Yellow)
        .text("Let thee be champions")];

    for fighter in &sorted_hof[0..sorted_hof.len().min(10)] {
        lines.push(
            Msg::new()
                .color(Color::Green)
                .text("-> ")
                .color(Color::Cyan)
                .text(fighter.0)
                .color(Color::Green)
                .text(" <- ")
                .color(Color::Yellow)
                .text(format!("({} pts)", fighter.1)),
        )
    }

    lines
}
