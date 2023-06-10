use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Write},
    ops::{Deref, DerefMut},
    time::{Duration, SystemTime},
};

use ircie::{
    format::{Color, Msg},
    irc_command::IrcCommand,
    system::IntoResponse,
    system_params::{AnyArguments, Arguments, Channel, Context, Res, ResMut},
    Irc, IrcPrefix,
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
    "casts hadoken at",
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

#[derive(Clone)]
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
    DeathMatch,
    FreeForAll,
    RoyalRumble,
    TeamBattle,
}

#[derive(Default, Clone, PartialEq, Eq)]
enum FightStatus {
    Happening,
    WaitingWho,
    WaitingChallengee(String, SystemTime),
    #[default]
    Idle,
}

#[derive(Default)]
struct Fight {
    status: FightStatus,
    channel: String,
    kind: FightKind,
    challengee: Option<String>,
    fighters: Vec<Fighter>,
}

impl Fight {
    pub fn reset(&mut self) {
        self.status = FightStatus::Idle;
        self.channel = "".to_owned();
        self.kind = FightKind::Duel;
        self.challengee = None;
        self.fighters.clear();
    }
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
        .add_interval_task(Duration::from_millis(10), fight)
        .await
        .add_event_system(IrcCommand::RPL_WHOREPLY, whoreply)
        .await
        .add_event_system(IrcCommand::RPL_ENDOFWHO, endofwho)
        .await
        .add_system("f", new_fight)
        .await
        .add_system("royalrumble", royal_rumble)
        .await
        .add_system("challenge", challenge)
        .await
        .add_system("accept", accept_challenge)
        .await
        .add_system("hof", show_hall_of_fame)
        .await
        .add_system("h", show_help)
        .await
        .add_system("s", show_status)
        .await
        .add_system("stop", stop)
        .await;

    irc.run().await?;
    Ok(())
}

fn fight(
    mut ctx: Context,
    mut fight: ResMut<Fight>,
    mut rng: ResMut<StdRng>,
    mut hall_of_fame: ResMut<HallOfFame>,
) {
    if let FightStatus::WaitingChallengee(nick, time) = &fight.status {
        if let Ok(elapsed) = time.elapsed() {
            if elapsed.as_secs() > 30 {
                ctx.privmsg(
                    &fight.channel,
                    &Msg::new()
                        .color(Color::Yellow)
                        .text(&format!("{} pussied out.", nick))
                        .to_string(),
                );

                fight.reset();
                return;
            }
        }
    }

    if fight.status != FightStatus::Happening {
        std::thread::sleep(Duration::from_millis(500));
        return;
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

        if fight.fighters.len() == 1 {
            ctx.privmsg(
                &fight.channel,
                &Msg::new()
                    .color(Color::Yellow)
                    .text("We have a winner! -> ")
                    .color(winners[0].color)
                    .text(&winners[0].nick)
                    .color(Color::Yellow)
                    .text(" <- !")
                    .to_string(),
            );
            if fight.kind == FightKind::DeathMatch {
                hall_of_fame.add_winner(&winners[0].nick);
            }
        } else {
            ctx.privmsg(
                &fight.channel,
                &Msg::new()
                    .color(Color::Yellow)
                    .text("We have winners!")
                    .to_string(),
            );

            for w in &winners {
                ctx.privmsg(
                    &fight.channel,
                    &Msg::new()
                        .color(Color::Yellow)
                        .text("-> ")
                        .color(w.color)
                        .text(&w.nick)
                        .color(Color::Yellow)
                        .text(" <-")
                        .to_string(),
                );
                if fight.kind == FightKind::DeathMatch {
                    hall_of_fame.add_winner(&w.nick);
                }
            }
        }

        for w in &winners {
            ctx.privmsg(
                &fight.channel,
                &Msg::new().text("!beer ").text(&w.nick).to_string(),
            );
        }

        ctx.mode(
            &fight.channel,
            &format!(
                "+{} {}",
                "v".repeat(winners.len()),
                winners.iter().map(|w| &w.nick).join(" ")
            ),
        );

        fight.fighters = vec![];
        hall_of_fame.save(SAVE_LOC).unwrap();
        fight.channel = "".to_owned();
        return;
    }

    let body_part = BODYPARTS.choose(&mut *rng).unwrap();
    let action = ACTIONS.choose(&mut *rng).unwrap();
    let damage = rng.gen::<f32>() * 75.;

    let attacker = fight.fighters.choose(&mut *rng).unwrap().clone();
    let chan = fight.channel.clone();

    let fucking_victim = if fight
        .fighters
        .iter()
        .filter(|f| f.team_idx == attacker.team_idx && f.nick != attacker.nick)
        .count()
        != 0
        && rng.gen_bool(1. / 4.)
    {
        let victim = fight
            .fighters
            .iter_mut()
            .filter(|f| f.team_idx == attacker.team_idx && f.nick != attacker.nick)
            .choose(&mut *rng)
            .unwrap();

        ctx.privmsg(
            &chan,
            &Msg::new()
                .color(Color::Yellow)
                .text("Oh no! ")
                .color(attacker.color)
                .text(&attacker.nick)
                .color(Color::Yellow)
                .text(&format!(" is fucking retarded and is attacking his mate!"))
                .to_string(),
        );
        victim
    } else {
        fight
            .fighters
            .iter_mut()
            .filter(|f| f.team_idx != attacker.team_idx)
            .choose(&mut *rng)
            .unwrap()
    };

    fucking_victim.health -= damage;
    let fucking_victim = fucking_victim.clone();

    ctx.privmsg(
        &fight.channel,
        &Msg::new()
            .color(attacker.color)
            .text(&attacker.nick)
            .reset()
            .text(&format!(" {} ", action))
            .color(fucking_victim.color)
            .text(&fucking_victim.nick)
            .reset()
            .text(&format!("'s {}! (-{:.2} hp)", body_part, damage))
            .to_string(),
    );

    if fucking_victim.health <= 0. {
        ctx.privmsg(
            &fight.channel,
            &Msg::new()
                .color(fucking_victim.color)
                .text(&fucking_victim.nick)
                .color(Color::Yellow)
                .text(" is lying dead!")
                .to_string(),
        );
        ctx.mode(&fight.channel, &format!("-v {}", fucking_victim.nick));

        if fight.kind == FightKind::DeathMatch {
            hall_of_fame.add_fucking_looser(&fucking_victim.nick);

            ctx.privmsg(
                "ChanServ",
                &format!(
                    "KICK {} {} {}",
                    fight.channel, fucking_victim.nick, "You fucking looser"
                ),
            );
        }
        fight.fighters.retain(|f| f.nick != fucking_victim.nick);
    }
    std::thread::sleep(Duration::from_millis(rng.gen_range(200..800)));
}

fn new_fight(
    arguments: AnyArguments,
    channel: Channel,
    mut fight: ResMut<Fight>,
    mut rng: ResMut<StdRng>,
) -> impl IntoResponse {
    if let Err(e) = check_idle(&fight.status) {
        return Err(e);
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
        _ => {}
    }

    fight.status = FightStatus::Happening;

    fight.channel = channel.to_owned();

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
        FightKind::RoyalRumble => init_msg.push(
            Msg::new()
                .color(Color::Yellow)
                .text("Tonight's fight is a RRRRRRROYAL RUMBLE!"),
        ),
        FightKind::DeathMatch => init_msg.push(
            Msg::new()
                .color(Color::Yellow)
                .text("Tonight's fight is a Death Match!"),
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

fn royal_rumble(
    _: Arguments<'_, 0>,
    channel: Channel,
    mut fight: ResMut<Fight>,
    mut ctx: Context,
) -> impl IntoResponse {
    if let Err(e) = check_idle(&fight.status) {
        return Err(e);
    }

    fight.status = FightStatus::WaitingWho;
    fight.channel = channel.to_owned();
    fight.kind = FightKind::RoyalRumble;

    ctx.who(&channel);

    Ok(())
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

fn show_help() -> impl IntoResponse {
    (
        false,
        vec![
            ",f <nick> <nick>            | duel fight",
            ",f <nick> ... vs <nick> ... | team battle",
            ",f <nick> <nick> <nick> ... | free for all",
            ",royalrumble                | chan wide free for all",
            ",challenge <nick>           | challenge someone to a deathmatch",
            ",accept                     | accept a challenge",
            ",s                          | show the current fight status",
            ",stop                       | stop the current fight",
            ",hof                        | hall of fame",
        ],
    )
}

fn show_status(fight: Res<Fight>) -> impl IntoResponse {
    if fight.status == FightStatus::Idle {
        return "Noone is fighting you bunch of pussies.".to_owned();
    }

    format!("{} fighters remaining", fight.fighters.len())
}

fn stop(prefix: IrcPrefix, mut fight: ResMut<Fight>) -> impl IntoResponse {
    if fight.kind == FightKind::DeathMatch {
        return Err("Can't stop a deathmatch");
    }
    if prefix.nick == "sht" {
        return Err("Not you you can't you grumpy nigga");
    }
    fight.reset();

    Ok(())
}

fn whoreply(
    arguments: AnyArguments,
    mut fight: ResMut<Fight>,
    mut rng: ResMut<StdRng>,
    mut ctx: Context,
) {
    match fight.kind {
        FightKind::DeathMatch => {
            if arguments[5] != fight.challengee.as_ref().unwrap() {
                return;
            };
            fight.status =
                FightStatus::WaitingChallengee(arguments[5].to_owned(), SystemTime::now());

            ctx.privmsg(
                &fight.channel,
                &Msg::new()
                    .color(Color::Yellow)
                    .text(format!(
                        "{} challenges {} to a deathmatch! you have 30s to ,accept or YOURE A BITCH",
                        fight.fighters[0].nick,
                        arguments[5]
                    ))
                    .to_string(),
            );
        }
        FightKind::RoyalRumble => {
            let color = COLORS.iter().choose(&mut *rng).unwrap();
            let idx = fight.fighters.len();

            fight
                .fighters
                .push(Fighter::new(arguments[5], *color.clone(), idx));
        }
        _ => {}
    }
}

fn endofwho(mut fight: ResMut<Fight>, rng: ResMut<StdRng>, mut ctx: Context) {
    match fight.kind {
        FightKind::DeathMatch => {
            if fight.status == FightStatus::WaitingWho {
                ctx.privmsg(
                    &fight.channel,
                    &Msg::new()
                        .color(Color::Yellow)
                        .text(format!(
                            "there's no {} here you stupid fuck",
                            fight.challengee.as_ref().unwrap()
                        ))
                        .to_string(),
                );
                fight.reset();
                return;
            }
        }
        FightKind::RoyalRumble => start_rumble(fight, rng, ctx),
        _ => {}
    }
}

fn start_rumble(mut fight: ResMut<Fight>, mut rng: ResMut<StdRng>, mut ctx: Context) {
    fight.status = FightStatus::Happening;

    let mut init_msg = vec![Msg::new()
        .color(Color::Yellow)
        .text("THE FIGHT IS ABOUT TO BEGIN! TAKE YOUR BETS!")];

    init_msg.push(
        Msg::new()
            .color(Color::Yellow)
            .text("Tonight's fight is a RRRRRRROYAL RUMBLE!"),
    );

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

    for line in init_msg {
        ctx.privmsg(&fight.channel, &line.to_string())
    }
}

fn challenge(
    prefix: IrcPrefix,
    arguments: Arguments<'_, 1>,
    channel: Channel,
    mut fight: ResMut<Fight>,
    mut ctx: Context,
    mut rng: ResMut<StdRng>,
) -> impl IntoResponse {
    if let Err(e) = check_idle(&fight.status) {
        return Err(e);
    }

    fight.status = FightStatus::WaitingWho;
    fight.channel = channel.to_owned();
    fight.kind = FightKind::DeathMatch;
    fight.challengee = Some(arguments[0].to_owned());

    let color = COLORS.iter().choose(&mut *rng).unwrap();
    fight
        .fighters
        .push(Fighter::new(prefix.nick, *color.clone(), 0));

    ctx.who(&channel);

    Ok(())
}

fn accept_challenge(
    prefix: IrcPrefix,
    _: Arguments<'_, 0>,
    mut fight: ResMut<Fight>,
    mut rng: ResMut<StdRng>,
) -> impl IntoResponse {
    let status = fight.status.clone();
    if let FightStatus::WaitingChallengee(nick, _) = status {
        if nick != prefix.nick {
            return Err("you haven't been challenged you stupid fuck.".to_owned());
        }

        let mut color = COLORS.iter().choose(&mut *rng).unwrap();
        while **color == fight.fighters[0].color {
            color = COLORS.iter().choose(&mut *rng).unwrap();
        }
        fight
            .fighters
            .push(Fighter::new(prefix.nick, Color::Cyan, 1));
        fight.status = FightStatus::Happening;

        let mut init_msg = vec![Msg::new()
            .color(Color::Yellow)
            .text("THE FIGHT IS ABOUT TO BEGIN! TAKE YOUR BETS!")];

        init_msg.push(
            Msg::new()
                .color(Color::Yellow)
                .text("Tonight's fight is a DEATH MATCH!"),
        );

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

        return Ok((false, init_msg));
    }

    return Err("you haven't been challenged you stupid fuck.".to_owned());
}

fn check_idle(status: &FightStatus) -> Result<(), String> {
    match status {
        FightStatus::Happening => Err("Shut up and watch the show".to_owned()),
        FightStatus::WaitingWho => Err("".to_owned()),
        FightStatus::WaitingChallengee(_, _) => {
            Err("A challenge is waiting to be accepted.".to_owned())
        }
        FightStatus::Idle => Ok(()),
    }
}
