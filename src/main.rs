
#[macro_use]
extern crate shrs;

use std::{
    fs,
    env,
    io::{stdout, BufWriter},
    process::Command,
};

use shrs::{prelude::{*, styled_buf::StyledBuf}, history::FileBackedHistory};
use shrs_command_timer::{CommandTimerPlugin, CommandTimerState};
use shrs_mux::{MuxPlugin, MuxState};
use shrs_cd_tools::{node::NodeJs, rust::CargoToml, git::Git, DirParsePlugin, DirParseState, default_prompt};
// use shrs_output_capture::OutputCapturePlugin;
// use shrs_run_context::RunContextPlugin;

struct MyPrompt;

impl Prompt for MyPrompt {
    fn prompt_left(&self, line_ctx: &mut LineCtx) -> StyledBuf {
        let indicator = match line_ctx.mode() {
            LineMode::Insert => String::from(">").cyan(),
            LineMode::Normal => String::from(":").yellow(),
        };
        if !line_ctx.lines.is_empty() {
            return styled! {" ", indicator, " "};
        }

        styled! {" ", @(blue)username(), " ", @(white,bold)top_pwd(), " ", indicator, " "}
    }
    fn prompt_right(&self, line_ctx: &mut LineCtx) -> StyledBuf {
        let time_str = line_ctx
            .ctx
            .state
            .get::<CommandTimerState>()
            .and_then(|x| x.command_time())
            .map(|x| format!("{x:?} "));

        let lang = line_ctx
            .ctx
            .state
            .get::<MuxState>()
            .map(|state| format!("{} ", state.get_lang()));

        let git_branch = line_ctx
            .ctx
            .state
            .get::<DirParseState>()
            .and_then(|state| state.get_module_metadata::<Git>("git"))
            .map(|git| format!("git:{} ", git.branch));

        if !line_ctx.lines.is_empty() {
            return styled! {""};
        }

        let project_indicator = default_prompt(line_ctx);

        styled! {@(bold,blue)git_branch, time_str, lang, project_indicator}
    }
}

fn main() {
    env_logger::init();

    let _out = BufWriter::new(stdout());

    let config_dir = dirs::home_dir().unwrap().as_path().join(".config/pinosh");
    fs::create_dir_all(config_dir.clone());

    let mut env = Env::default();
    env.load();
    env.set("SHELL_NAME", "pinosh");

    let builtins = Builtins::default();

    let path_string = env.get("PATH").unwrap().to_string();
    let mut completer = DefaultCompleter::default();
    completer.register(Rule::new(
        Pred::new(cmdname_pred),
        Box::new(cmdname_action(path_string)),
    ));
    completer.register(Rule::new(
        Pred::new(cmdname_pred),
        Box::new(builtin_cmdname_action(&builtins)),
    ));

    let menu = DefaultMenu::default();

    let history_file = config_dir.as_path().join("history");
    let history = FileBackedHistory::new(history_file).expect("Could not open history file");

    let highlighter = SyntaxHighlighter::new(SyntaxTheme::default());

    let keybinding = keybindings! {
        |sh, ctx, rt|
        "C-l" => ("clear the screen", { Command::new("clear").spawn() }),
        "C-f" => ("fuzzy search", {
            
            let Ok(search_dirs) = env::var("FUZZY_DIRS") else {
                eprintln!("FUZZY_DIRS env var not specified");
                return;
            };

            let dir = Command::new("fdfind")
                .args(&[".", "-t", "d", &search_dirs, "|", "fzf"])
                .output()
                .unwrap()
                .stdout;
            let dir = String::from_utf8(dir).unwrap();

            sh.builtins.get("cd").unwrap().run(sh, ctx, rt, &vec![dir]).unwrap();
        }),
    };

    let prompt = MyPrompt;

    let readline = LineBuilder::default()
        .with_completer(completer)
        .with_menu(menu)
        .with_highlighter(highlighter)
        .with_prompt(prompt)
        .build()
        .expect("Could not construct readline");

    let alias = Alias::from_iter([
        ("ls", "ls --color=auto"),
        ("l", "ls --color=auto"),
        ("c", "cd"),
        ("g", "git"),
        ("v", "vim"),
        ("V", "nvim"),
        ("la", "ls -a --color=auto"),
        ("t", "task"),
    ]);

    let startup_msg: HookFn<StartupCtx> = |_sh: &Shell,
                                           _sh_ctx: &mut Context,
                                           _sh_rt: &mut Runtime,
                                           _ctx: &StartupCtx|
     -> anyhow::Result<()> {
        let welcome_str = format!(
            r#"
         _                 _     
   _ __ (_)_ __   ___  ___| |__  
  | '_ \| | '_ \ / _ \/ __| '_ \ 
  | |_) | | | | | (_) \__ \ | | |
  | .__/|_|_| |_|\___/|___/_| |_|
  |_| pinosaur's shell

  ###############################
"#,
        );

        println!("{welcome_str}");
        Ok(())
    };
    let mut hooks = Hooks::new();
    hooks.insert(startup_msg);

    let myshell = ShellBuilder::default()
        .with_hooks(hooks)
        .with_env(env)
        .with_alias(alias)
        .with_readline(readline)
        .with_history(history)
        .with_keybinding(keybinding)
        // .with_plugin(OutputCapturePlugin)
        .with_plugin(CommandTimerPlugin)
        // .with_plugin(RunContextPlugin)
        .with_plugin(DirParsePlugin::new())
        .with_plugin(MuxPlugin::new())
        .build()
        .expect("Could not construct shell");

    myshell.run();
}
