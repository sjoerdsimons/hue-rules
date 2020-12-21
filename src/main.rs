use anyhow::Result;
use huelib::resource::rule::{Action, Condition, Creator, Modifier};
use huelib::Bridge;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::net::IpAddr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Dump {
    #[structopt(short, long)]
    out: Option<String>,
}

#[derive(StructOpt, Debug)]
struct Upload {
    input: String,
}

#[derive(StructOpt, Debug)]
enum Command {
    Dump(Dump),
    Upload(Upload),
}

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(short, long, env = "HUECTL_BRIDGE_IP")]
    ip: IpAddr,
    #[structopt(short, long, env = "HUECTL_BRIDGE_USERNAME")]
    user: String,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct Rule {
    id: String,
    name: String,
    actions: Vec<Action>,
    conditions: Vec<Condition>,
}

impl From<huelib::resource::Rule> for Rule {
    fn from(r: huelib::resource::Rule) -> Rule {
        Rule {
            id: r.id,
            name: r.name,
            conditions: r.conditions,
            actions: r.actions,
        }
    }
}

fn get_rules(bridge: &Bridge) -> Result<Vec<Rule>> {
    let rules = bridge.get_all_rules()?;
    let mut rules: Vec<Rule> = rules.iter().map(|r| r.clone().into()).collect();

    rules.sort_by_key(|r| r.id.parse::<i32>().unwrap_or(0));
    Ok(rules)
}

fn cmd_upload(bridge: Bridge, upload: Upload) -> Result<()> {
    let current = get_rules(&bridge)?;

    let file = File::open(upload.input)?;
    let mut update: Vec<Rule> = serde_yaml::from_reader(file)?;
    update.sort_by_key(|r| r.id.parse::<i32>().unwrap_or(0));

    for rule in update {
        if let Some(r) = current.iter().find(|r| r.id == rule.id) {
            if r == &rule {
                println!("Unchanged {} -- {}", rule.id, rule.name);
            } else {
                println!("Changed {} -- {}", rule.id, rule.name);
                let modifier = Modifier::new()
                    .with_name(rule.name)
                    .with_actions(rule.actions)
                    .with_conditions(rule.conditions);
                let r = bridge.set_rule(rule.id, &modifier)?;
                println!("Rule updated: {:?}", r);
            }
        } else {
            println!("New {} -- {}", rule.id, rule.name);
            let creator = Creator::new(rule.conditions, rule.actions).with_name(rule.name);
            let id = bridge.create_rule(&creator)?;
            println!("Created with id: {}", id);
        }
    }

    Ok(())
}

fn cmd_dump(bridge: Bridge, dump: Dump) -> Result<()> {
    let rules = get_rules(&bridge)?;

    if let Some(out) = dump.out {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out)?;
        serde_yaml::to_writer(&file, &rules)?;
    } else {
        println!("{}", serde_yaml::to_string(&rules)?);
    }

    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let bridge = Bridge::new(opt.ip, opt.user);
    let config = bridge.get_config()?;

    println!("Connected to {}", config.name);
    match opt.command {
        Command::Dump(d) => cmd_dump(bridge, d)?,
        Command::Upload(u) => cmd_upload(bridge, u)?,
    }

    Ok(())
}
