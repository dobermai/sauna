use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use klafs_api::{ClientConfig, DebugConfig, KlafsClient, SaunaInfo, SaunaMode, SaunaStatus, StatusCode};
use std::path::{Path, PathBuf};

mod config;
mod profiles;

use config::Config;
use profiles::{Profile, Profiles};

#[derive(Parser)]
#[command(name = "sauna")]
#[command(author, version, about = "Control your Klafs sauna from the command line")]
#[command(propagate_version = true)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable HTTP debug logging (writes to file)
    #[arg(long, global = true)]
    debug: bool,

    /// Debug log file path
    #[arg(long, global = true, default_value = "klafs-debug.log")]
    debug_file: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Login to your Klafs account and store credentials
    Login {
        /// Email address for your Klafs account
        #[arg(short, long)]
        username: Option<String>,

        /// Password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Logout and remove stored credentials
    Logout,

    /// List all saunas registered to your account
    Saunas {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Configure the CLI (set default sauna ID, PIN, etc.)
    Config {
        /// Set the default sauna ID
        #[arg(long)]
        sauna_id: Option<String>,

        /// Set the PIN for power control (stored securely)
        #[arg(long)]
        pin: Option<String>,

        /// Show current configuration
        #[arg(long)]
        show: bool,
    },

    /// Get the current status of your sauna
    Status {
        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Power on the sauna (requires PIN)
    PowerOn {
        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,

        /// PIN for power control
        #[arg(short, long)]
        pin: Option<String>,

        /// Schedule start time in HH:MM format (e.g., --at 18:30)
        #[arg(long = "at")]
        schedule: Option<String>,
    },

    /// Power off the sauna (no PIN required)
    PowerOff {
        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,
    },

    /// Set the target temperature
    SetTemp {
        /// Target temperature in °C (10-100 for Sauna, 40-75 for Sanarium)
        temperature: i32,

        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,
    },

    /// Set the operating mode
    SetMode {
        /// Mode: sauna, sanarium, or infrared
        mode: String,

        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,
    },

    /// Set the humidity level (Sanarium mode only)
    SetHumidity {
        /// Humidity level (1-10)
        level: i32,

        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,
    },

    /// Set or clear the scheduled start time without starting the sauna
    Schedule {
        /// Start time in HH:MM format (e.g., 18:30). Omit to clear schedule.
        time: Option<String>,

        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,

        /// Clear the scheduled time
        #[arg(long)]
        clear: bool,
    },

    /// Manage saved profiles for quick sauna configuration
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },

    /// Configure multiple sauna settings in one command
    Configure {
        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,

        /// Target temperature in °C (for current mode)
        #[arg(short, long)]
        temp: Option<i32>,

        /// Humidity level (1-10, sanarium mode only)
        #[arg(long)]
        humidity: Option<i32>,

        /// Scheduled start time in HH:MM format
        #[arg(long)]
        time: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// Create a new profile
    Create {
        /// Name for the profile
        name: String,

        /// Operating mode: sauna, sanarium, or infrared
        #[arg(short, long)]
        mode: String,

        /// Target temperature in °C
        #[arg(short, long)]
        temp: i32,

        /// Humidity level (1-10, sanarium mode only)
        #[arg(long)]
        humidity: Option<i32>,

        /// Infrared level (1-10, infrared mode only)
        #[arg(long)]
        ir_level: Option<i32>,
    },

    /// List all saved profiles
    List,

    /// Apply a profile to the sauna
    Apply {
        /// Profile name to apply
        name: String,

        /// Sauna ID (uses default from config if not provided)
        #[arg(short, long)]
        sauna_id: Option<String>,

        /// Also start the sauna after applying (requires PIN)
        #[arg(long)]
        start: bool,

        /// PIN for power control (only with --start)
        #[arg(short, long)]
        pin: Option<String>,
    },

    /// Delete a profile
    Delete {
        /// Profile name to delete
        name: String,
    },

    /// Show details of a profile
    Show {
        /// Profile name to show
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("klafs=debug")
            .init();
    }

    match cli.command {
        Commands::Login { username, password } => {
            cmd_login(username, password, cli.debug, &cli.debug_file).await
        }
        Commands::Logout => cmd_logout().await,
        Commands::Saunas { json } => cmd_saunas(json, cli.debug, &cli.debug_file).await,
        Commands::Config {
            sauna_id,
            pin,
            show,
        } => cmd_config(sauna_id, pin, show).await,
        Commands::Status { sauna_id, json } => {
            cmd_status(sauna_id, json, cli.debug, &cli.debug_file).await
        }
        Commands::PowerOn { sauna_id, pin, schedule } => {
            cmd_power_on(sauna_id, pin, schedule, cli.debug, &cli.debug_file).await
        }
        Commands::PowerOff { sauna_id } => {
            cmd_power_off(sauna_id, cli.debug, &cli.debug_file).await
        }
        Commands::SetTemp {
            temperature,
            sauna_id,
        } => cmd_set_temp(temperature, sauna_id, cli.debug, &cli.debug_file).await,
        Commands::SetMode { mode, sauna_id } => {
            cmd_set_mode(mode, sauna_id, cli.debug, &cli.debug_file).await
        }
        Commands::SetHumidity { level, sauna_id } => {
            cmd_set_humidity(level, sauna_id, cli.debug, &cli.debug_file).await
        }
        Commands::Schedule { time, sauna_id, clear } => {
            cmd_schedule(time, sauna_id, clear, cli.debug, &cli.debug_file).await
        }
        Commands::Profile { command } => cmd_profile(command, cli.debug, &cli.debug_file).await,
        Commands::Configure {
            sauna_id,
            temp,
            humidity,
            time,
        } => cmd_configure(sauna_id, temp, humidity, time, cli.debug, &cli.debug_file).await,
    }
}

fn create_client_config(debug: bool, debug_file: &Path) -> ClientConfig {
    if debug {
        ClientConfig {
            debug: DebugConfig::enabled().with_log_file(debug_file.to_path_buf()),
            ..Default::default()
        }
    } else {
        ClientConfig::default()
    }
}

async fn cmd_login(
    username: Option<String>,
    password: Option<String>,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let mut config = Config::load()?;

    // Get username
    let username = match username {
        Some(u) => u,
        None => {
            print!("Email: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        }
    };

    if username.is_empty() {
        bail!("Username cannot be empty");
    }

    // Get password
    let password = match password {
        Some(p) => p,
        None => rpassword::prompt_password("Password: ")?,
    };

    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    // Attempt login
    println!("{}", "Logging in...".dimmed());

    let client_config = create_client_config(debug, debug_file);
    let client = KlafsClient::with_config(client_config);

    client.login(&username, &password).await.with_context(|| {
        format!(
            "Login failed. {} Klafs locks accounts after 3 failed attempts!",
            "Warning:".yellow().bold()
        )
    })?;

    // Store credentials
    Config::store_password(&username, &password)?;
    config.username = Some(username.clone());
    config.save()?;

    println!(
        "{} Logged in as {}",
        "Success!".green().bold(),
        username.cyan()
    );
    println!(
        "{}",
        "Credentials stored securely in system keyring.".dimmed()
    );

    if debug {
        println!(
            "{}",
            format!("Debug log written to: {}", debug_file.display()).dimmed()
        );
    }

    Ok(())
}

async fn cmd_logout() -> Result<()> {
    let mut config = Config::load()?;

    if let Some(username) = &config.username {
        Config::delete_password(username)?;
        println!("Removed credentials for {}", username.cyan());
    }

    config.username = None;
    config.save()?;

    println!("{} Logged out successfully.", "Done.".green().bold());

    Ok(())
}

async fn cmd_saunas(json: bool, debug: bool, debug_file: &Path) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let saunas = client.list_saunas().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&saunas)?);
    } else {
        print_saunas(&saunas, &config);
    }

    Ok(())
}

fn print_saunas(saunas: &[SaunaInfo], config: &Config) {
    if saunas.is_empty() {
        println!("{}", "No saunas found on this account.".yellow());
        println!(
            "{}",
            "Make sure your sauna is registered in the Klafs app.".dimmed()
        );
        return;
    }

    println!("{}", "Registered Saunas".bold().underline());
    println!();

    for sauna in saunas {
        let is_default = config
            .sauna_id
            .as_ref()
            .map(|id| id == &sauna.id)
            .unwrap_or(false);

        let marker = if is_default {
            "*".green().bold()
        } else {
            " ".normal()
        };

        println!("{} {}", marker, sauna.name.cyan().bold());
        println!("    ID: {}", sauna.id.dimmed());

        if is_default {
            println!("    {}", "(default)".green());
        }
        println!();
    }

    println!(
        "{}",
        "Use 'klafs config --sauna-id <ID>' to set a default.".dimmed()
    );
}

async fn cmd_config(sauna_id: Option<String>, pin: Option<String>, show: bool) -> Result<()> {
    let mut config = Config::load()?;

    if show {
        println!("{}", "Current configuration:".bold());
        println!(
            "  Username:  {}",
            config
                .username
                .as_deref()
                .unwrap_or("(not set)")
                .cyan()
        );
        println!(
            "  Sauna ID:  {}",
            config
                .sauna_id
                .as_deref()
                .unwrap_or("(not set)")
                .cyan()
        );
        println!(
            "  Config:    {}",
            Config::config_path()?.display().to_string().dimmed()
        );

        if let Some(ref sid) = config.sauna_id {
            let has_pin = Config::get_pin(sid)?.is_some();
            println!(
                "  PIN:       {}",
                if has_pin {
                    "(stored)".green()
                } else {
                    "(not set)".dimmed()
                }
            );
        }

        return Ok(());
    }

    let mut changed = false;

    if let Some(sid) = sauna_id {
        config.sauna_id = Some(sid.clone());
        println!("Set default sauna ID to {}", sid.cyan());
        changed = true;
    }

    if let Some(pin_value) = pin {
        let sid = config
            .sauna_id
            .as_ref()
            .context("Set a sauna ID first with --sauna-id")?;
        Config::store_pin(sid, &pin_value)?;
        println!("{} PIN stored securely.", "Done.".green().bold());
        changed = true;
    }

    if changed {
        config.save()?;
    } else {
        println!(
            "{}",
            "No changes made. Use --show to view config or provide options to set.".dimmed()
        );
    }

    Ok(())
}

async fn cmd_status(
    sauna_id: Option<String>,
    json: bool,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id
        .or(config.sauna_id)
        .context("No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'")?;

    let status = client.get_status(&sauna_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        print_status(&status);
    }

    Ok(())
}

fn print_status(status: &SaunaStatus) {
    println!("{}", "Sauna Status".bold().underline());
    println!();

    // Connection status
    let conn_status = if status.is_connected {
        "Connected".green()
    } else {
        "Disconnected".red()
    };
    println!("  Connection:     {}", conn_status);

    // Power status
    let power_status = if status.is_powered_on {
        "On".green().bold()
    } else {
        "Off".dimmed()
    };
    println!("  Power:          {}", power_status);

    // Status
    let status_display = match status.status() {
        StatusCode::Off => "Off".dimmed(),
        StatusCode::HeatingUp => "Heating Up".yellow(),
        StatusCode::Ready => "Ready".green().bold(),
        StatusCode::Standby => "Standby".blue(),
        StatusCode::Unknown(code) => format!("Unknown ({})", code).red().normal(),
    };
    println!("  Status:         {}", status_display);

    if status.is_ready_for_use {
        println!("                  {}", "Ready for use!".green().bold());
    }

    println!();

    // Mode
    let mode = if status.sauna_selected {
        "Sauna".cyan()
    } else if status.sanarium_selected {
        "Sanarium".magenta()
    } else if status.ir_selected {
        "Infrared".yellow()
    } else {
        "None".dimmed()
    };
    println!("  Mode:           {}", mode);

    // Temperature
    println!(
        "  Current Temp:   {}°C",
        format!("{}", status.current_temperature).white().bold()
    );
    println!(
        "  Target Temp:    {}°C",
        format!("{}", status.target_temperature()).cyan()
    );

    // Humidity (if in Sanarium mode)
    if status.sanarium_selected {
        println!(
            "  Current Humid:  {}%",
            format!("{}", status.current_humidity).white().bold()
        );
        println!(
            "  Target Humid:   Level {}",
            format!("{}", status.selected_hum_level).cyan()
        );
    }

    // Timer
    if status.show_bathing_hour || status.is_powered_on {
        println!();
        println!(
            "  Remaining Time: {}",
            status.remaining_time().yellow()
        );
    }

    // Scheduled start
    if status.selected_hour > 0 || status.selected_minute > 0 {
        println!(
            "  Scheduled:      {:02}:{:02}",
            status.selected_hour, status.selected_minute
        );
    }

    println!();
}

async fn cmd_power_on(
    sauna_id: Option<String>,
    pin: Option<String>,
    schedule: Option<String>,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id.or(config.sauna_id).context(
        "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
    )?;

    let pin = match pin {
        Some(p) => p,
        None => Config::get_pin(&sauna_id)?
            .context("No PIN provided. Use --pin or store it with 'klafs config --pin <PIN>'")?,
    };

    // Parse optional schedule time
    let schedule_time = match schedule {
        Some(time_str) => {
            let parts: Vec<&str> = time_str.split(':').collect();
            if parts.len() != 2 {
                bail!("Invalid time format '{}'. Use HH:MM format (e.g., 18:30)", time_str);
            }
            let hour: i32 = parts[0]
                .parse()
                .with_context(|| format!("Invalid hour: {}", parts[0]))?;
            let minute: i32 = parts[1]
                .parse()
                .with_context(|| format!("Invalid minute: {}", parts[1]))?;
            Some((hour, minute))
        }
        None => None,
    };

    match &schedule_time {
        Some((hour, minute)) => {
            println!("{}", format!("Scheduling sauna to start at {:02}:{:02}...", hour, minute).dimmed());
        }
        None => {
            println!("{}", "Powering on sauna...".dimmed());
        }
    }

    client.power_on(&sauna_id, &pin, schedule_time).await?;

    match schedule_time {
        Some((hour, minute)) => {
            println!(
                "{} Sauna scheduled to start at {:02}:{:02}",
                "Success!".green().bold(),
                hour,
                minute
            );
        }
        None => {
            println!("{} Sauna is powering on!", "Success!".green().bold());
        }
    }

    Ok(())
}

async fn cmd_power_off(
    sauna_id: Option<String>,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id.or(config.sauna_id).context(
        "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
    )?;

    println!("{}", "Powering off sauna...".dimmed());

    client.power_off(&sauna_id).await?;

    println!("{} Sauna powered off.", "Success!".green().bold());

    Ok(())
}

async fn cmd_set_temp(
    temperature: i32,
    sauna_id: Option<String>,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id.or(config.sauna_id).context(
        "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
    )?;

    println!("{}", format!("Setting temperature to {}°C...", temperature).dimmed());

    client.set_temperature(&sauna_id, temperature).await?;

    println!(
        "{} Temperature set to {}°C",
        "Success!".green().bold(),
        temperature.to_string().cyan()
    );

    Ok(())
}

async fn cmd_set_mode(
    mode: String,
    sauna_id: Option<String>,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id.or(config.sauna_id).context(
        "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
    )?;

    let sauna_mode = match mode.to_lowercase().as_str() {
        "sauna" => SaunaMode::Sauna,
        "sanarium" => SaunaMode::Sanarium,
        "infrared" | "ir" => SaunaMode::Infrared,
        _ => bail!("Invalid mode '{}'. Use: sauna, sanarium, or infrared", mode),
    };

    println!("{}", format!("Setting mode to {}...", mode).dimmed());

    client.set_mode(&sauna_id, sauna_mode).await?;

    println!(
        "{} Mode set to {}",
        "Success!".green().bold(),
        mode.cyan()
    );

    Ok(())
}

async fn cmd_set_humidity(
    level: i32,
    sauna_id: Option<String>,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id.or(config.sauna_id).context(
        "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
    )?;

    println!("{}", format!("Setting humidity level to {}...", level).dimmed());

    client.set_humidity(&sauna_id, level).await?;

    println!(
        "{} Humidity level set to {}",
        "Success!".green().bold(),
        level.to_string().cyan()
    );

    Ok(())
}

async fn cmd_schedule(
    time: Option<String>,
    sauna_id: Option<String>,
    clear: bool,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id.or(config.sauna_id).context(
        "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
    )?;

    // Determine if we're setting or clearing the schedule
    let schedule_time = if clear {
        if time.is_some() {
            bail!("Cannot use --clear with a time argument");
        }
        None
    } else {
        match time {
            Some(time_str) => {
                // Parse time in HH:MM format
                let parts: Vec<&str> = time_str.split(':').collect();
                if parts.len() != 2 {
                    bail!("Invalid time format '{}'. Use HH:MM format (e.g., 18:30)", time_str);
                }
                let hour: i32 = parts[0]
                    .parse()
                    .with_context(|| format!("Invalid hour: {}", parts[0]))?;
                let minute: i32 = parts[1]
                    .parse()
                    .with_context(|| format!("Invalid minute: {}", parts[1]))?;
                Some((hour, minute))
            }
            None => None, // No time provided and no --clear, so clear the schedule
        }
    };

    match &schedule_time {
        Some((hour, minute)) => {
            println!("{}", format!("Setting schedule to {:02}:{:02}...", hour, minute).dimmed());
        }
        None => {
            println!("{}", "Clearing schedule...".dimmed());
        }
    }

    client.set_selected_time(&sauna_id, schedule_time).await?;

    match schedule_time {
        Some((hour, minute)) => {
            println!(
                "{} Schedule set to {:02}:{:02}",
                "Success!".green().bold(),
                hour,
                minute
            );
        }
        None => {
            println!("{} Schedule cleared.", "Success!".green().bold());
        }
    }

    Ok(())
}

async fn cmd_configure(
    sauna_id: Option<String>,
    temp: Option<i32>,
    humidity: Option<i32>,
    time: Option<String>,
    debug: bool,
    debug_file: &Path,
) -> Result<()> {
    let config = Config::load()?;
    let client = create_authenticated_client(&config, debug, debug_file).await?;

    let sauna_id = sauna_id.or(config.sauna_id).context(
        "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
    )?;

    // Parse time if provided
    let (hour, minute) = match time {
        Some(time_str) => {
            let parts: Vec<&str> = time_str.split(':').collect();
            if parts.len() != 2 {
                bail!("Invalid time format '{}'. Use HH:MM format (e.g., 18:30)", time_str);
            }
            let hour: i32 = parts[0]
                .parse()
                .with_context(|| format!("Invalid hour: {}", parts[0]))?;
            let minute: i32 = parts[1]
                .parse()
                .with_context(|| format!("Invalid minute: {}", parts[1]))?;
            (Some(hour), Some(minute))
        }
        None => (None, None),
    };

    // Check that at least one option is provided
    if temp.is_none() && humidity.is_none() && hour.is_none() {
        bail!("No configuration options provided. Use --temp, --humidity, or --time.");
    }

    // Build description of changes
    let mut changes = Vec::new();
    if let Some(t) = temp {
        changes.push(format!("temperature {}°C", t));
    }
    if let Some(h) = humidity {
        changes.push(format!("humidity level {}", h));
    }
    if let (Some(h), Some(m)) = (hour, minute) {
        changes.push(format!("start time {:02}:{:02}", h, m));
    }

    println!(
        "{}",
        format!("Configuring: {}...", changes.join(", ")).dimmed()
    );

    // Note: The configure method takes both sauna and sanarium temperature
    // For simplicity, we set both to the same value if temp is provided
    // The API will use the appropriate one based on current mode
    client
        .configure(&sauna_id, temp, temp, humidity, hour, minute)
        .await?;

    println!(
        "{} Configuration applied: {}",
        "Success!".green().bold(),
        changes.join(", ")
    );

    Ok(())
}

async fn cmd_profile(command: ProfileCommands, debug: bool, debug_file: &Path) -> Result<()> {
    match command {
        ProfileCommands::Create {
            name,
            mode,
            temp,
            humidity,
            ir_level,
        } => {
            let mut profiles = Profiles::load()?;

            if profiles.exists(&name) {
                bail!("Profile '{}' already exists. Delete it first or use a different name.", name);
            }

            let profile = Profile::new(&mode, temp, humidity, ir_level)?;
            profiles.set(&name, profile.clone());
            profiles.save()?;

            println!(
                "{} Created profile '{}': {}",
                "Success!".green().bold(),
                name.cyan(),
                profile.description()
            );
        }

        ProfileCommands::List => {
            let profiles = Profiles::load()?;
            let names = profiles.list();

            if names.is_empty() {
                println!("{}", "No profiles saved.".dimmed());
                println!(
                    "{}",
                    "Use 'klafs profile create <name> --mode <mode> --temp <temp>' to create one.".dimmed()
                );
                return Ok(());
            }

            println!("{}", "Saved Profiles".bold().underline());
            println!();

            for name in names {
                if let Some(profile) = profiles.get(name) {
                    println!("  {} {}", "*".cyan(), name.cyan().bold());
                    println!("      {}", profile.description().dimmed());
                }
            }
            println!();
        }

        ProfileCommands::Show { name } => {
            let profiles = Profiles::load()?;

            match profiles.get(&name) {
                Some(profile) => {
                    println!("{} {}", "Profile:".bold(), name.cyan().bold());
                    println!("  Mode:        {}", profile.mode.cyan());
                    println!("  Temperature: {}°C", profile.temperature.to_string().cyan());
                    if let Some(hum) = profile.humidity {
                        println!("  Humidity:    {}", hum.to_string().cyan());
                    }
                    if let Some(ir) = profile.ir_level {
                        println!("  IR Level:    {}", ir.to_string().cyan());
                    }
                }
                None => {
                    bail!("Profile '{}' not found", name);
                }
            }
        }

        ProfileCommands::Apply {
            name,
            sauna_id,
            start,
            pin,
        } => {
            let profiles = Profiles::load()?;
            let config = Config::load()?;

            let profile = profiles
                .get(&name)
                .with_context(|| format!("Profile '{}' not found", name))?;

            let sauna_id = sauna_id.or(config.sauna_id.clone()).context(
                "No sauna ID provided. Use --sauna-id or set a default with 'klafs config --sauna-id <ID>'",
            )?;

            let client = create_authenticated_client(&config, debug, debug_file).await?;

            // First set the mode
            let sauna_mode = match profile.mode.as_str() {
                "sauna" => SaunaMode::Sauna,
                "sanarium" => SaunaMode::Sanarium,
                "infrared" => SaunaMode::Infrared,
                _ => bail!("Invalid mode in profile: {}", profile.mode),
            };

            println!(
                "{}",
                format!("Applying profile '{}'...", name).dimmed()
            );

            // Set mode
            client.set_mode(&sauna_id, sauna_mode).await?;

            // Apply temperature and levels using FavoriteSelected
            let humidity_level = profile.humidity.unwrap_or(0);
            let ir_level = profile.ir_level.unwrap_or(0);

            client
                .apply_favorite(&sauna_id, profile.temperature, humidity_level, ir_level)
                .await?;

            println!(
                "{} Profile '{}' applied: {}",
                "Success!".green().bold(),
                name.cyan(),
                profile.description()
            );

            // Optionally start the sauna
            if start {
                let pin = match pin {
                    Some(p) => p,
                    None => Config::get_pin(&sauna_id)?
                        .context("No PIN provided. Use --pin or store it with 'klafs config --pin <PIN>'")?,
                };

                println!("{}", "Starting sauna...".dimmed());
                client.power_on(&sauna_id, &pin, None).await?;
                println!("{} Sauna is powering on!", "Success!".green().bold());
            }
        }

        ProfileCommands::Delete { name } => {
            let mut profiles = Profiles::load()?;

            if profiles.remove(&name).is_none() {
                bail!("Profile '{}' not found", name);
            }

            profiles.save()?;

            println!(
                "{} Profile '{}' deleted.",
                "Done.".green().bold(),
                name.cyan()
            );
        }
    }

    Ok(())
}

/// Create an authenticated Klafs client using stored credentials
async fn create_authenticated_client(
    config: &Config,
    debug: bool,
    debug_file: &Path,
) -> Result<KlafsClient> {
    let username = config
        .username
        .as_ref()
        .context("Not logged in. Run 'klafs login' first.")?;

    let password = Config::get_password(username)?
        .context("Password not found in keyring. Run 'klafs login' again.")?;

    let client_config = create_client_config(debug, debug_file);
    let client = KlafsClient::with_config(client_config);
    client.login(username, &password).await?;

    if debug {
        eprintln!(
            "{}",
            format!("Debug logging enabled. Writing to: {}", debug_file.display()).dimmed()
        );
    }

    Ok(client)
}
