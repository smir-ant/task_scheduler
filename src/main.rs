use clap::{Parser, Subcommand};
use chrono::{DateTime, Utc, NaiveDateTime, Duration as ChronoDuration};
use sqlx::{SqlitePool, FromRow};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use anyhow::Result;
use std::fs::OpenOptions;
use cron::Schedule;
use std::str::FromStr;

/// CLI структура
#[derive(Parser)]
#[command(
    name = "scheduler",
    about = "Асинхронный кроссплатформенный планировщик задач",
    long_about = "Примеры использования (всё время в UTC+0):
[инициализация БД] 
└─ scheduler init
[запуск python с аргументами каждые 5 минут, начиная 2025-05-20 10:00:00] (add-interval только в минутах)
└─ scheduler add-interval --name python_every5Min --interval 5 --start 2025-05-20T10:00:00 --cmd \"python3 /path/to/script --arg1\"
[каждые 7 секунд] 
└─ scheduler add-cron --name every7s --expr \"1/7 * * * * *\" --cmd \"echo every7Sec\"
[каждый день в 08:59:30]
└─ scheduler add-cron --name at_08_59_30 --expr \"30 59 8 * * *\" --cmd \"echo at_08_59_30\"
[раз в первый день месяца в полночь]
└─ scheduler add-cron --name monthly --expr \"0 0 0 1 * *\" --cmd \"echo monthly\"
[5, 15 и 45 секунды каждой минуты]
└─ scheduler add-cron --name special_second --expr \"5,15,45 * * * * *\" --cmd \"echo 5_15_45Sec\"
[каждый час с 15-го по 20-е число]
└─ scheduler add-cron --name midmonth_hourly --expr \"0 0 * 15-20 * *\" --cmd \"echo midmonth_hourly\"

Cron-выражение состоит из 6 полей, разделенных пробелами:
1. Секунды (0–59 или списки/диапазоны/шаги)
2. Минуты (0–59)
3. Часы (0–23)
4. День месяца (1–31)
5. Месяц (1–12 или Jan–Dec)
6. День недели (0–6, где 0=Sun или Mon–Sun)
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Подкоманды
#[derive(Subcommand)]
enum Commands {
    /// Инициализировать БД
    Init,
    /// Запустить демон
    Run,
    /// Добавить интервальную задачу
    AddInterval {
        /// Уникальное имя задачи
        #[arg(long)]
        name: String,
        /// Интервал в минутах
        #[arg(long)]
        interval: i64,
        /// Время старта UTC (e.g. 2025-05-20T10:00:00)
        #[arg(long)]
        start: String,
        /// Команда с аргументами в кавычках
        #[arg(long)]
        cmd: String,
    },
    /// Добавить задачу с cron-выражением
    AddCron {
        /// Уникальное имя задачи
        #[arg(long)]
        name: String,
        /// Cron-выражение (секунды минуты часы дни_месяца месяца дни_недели)
        #[arg(long)]
        expr: String,
        /// Команда с аргументами в кавычках
        #[arg(long)]
        cmd: String,
    },
    /// Список задач
    List,
    /// Удалить задачу по имени
    Remove {
        /// Имя задачи
        #[arg(long)]
        name: String,
    },
}

/// Модель задачи
#[derive(FromRow, Debug)]
struct Task {
    id: i64,
    name: String,
    cmd: String,
    schedule_type: String,
    interval_minutes: Option<i64>,
    start_time: Option<String>,
    cron_expr: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Commands::Init = &cli.command {
        init_db().await?;
        println!("БД и таблицы созданы.");
        return Ok(());
    }
    // Проверка наличия файла БД
    if OpenOptions::new().read(true).open("tasks.db").is_err() {
        eprintln!("tasks.db не найден. Запустите: scheduler init");
        std::process::exit(1);
    }
    let db = SqlitePool::connect("sqlite://tasks.db").await?;
    match cli.command {
        Commands::Run => run_scheduler(&db).await?,
        Commands::AddInterval { name, interval, start, cmd } => {
            add_interval(&db, &name, interval, &start, &cmd).await?
        }
        Commands::AddCron { name, expr, cmd } => {
            add_cron(&db, &name, &expr, &cmd).await?
        }
        Commands::List => list_tasks(&db).await?,
        Commands::Remove { name } => remove_task(&db, &name).await?,
        Commands::Init => unreachable!(),
    }
    Ok(())
}

async fn init_db() -> Result<()> {
    let _ = OpenOptions::new().create(true).write(true).open("tasks.db")?;
    let db = SqlitePool::connect("sqlite://tasks.db").await?;
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            cmd TEXT NOT NULL,
            schedule_type TEXT NOT NULL,
            interval_minutes INTEGER,
            start_time TEXT,
            cron_expr TEXT
        )"#,
    )
    .execute(&db)
    .await?;
    Ok(())
}

async fn add_interval(
    db: &SqlitePool,
    name: &str,
    interval: i64,
    start: &str,
    cmd: &str,
) -> Result<()> {
    // Парсим дату-время
    let naive = NaiveDateTime::parse_from_str(start, "%Y-%m-%dT%H:%M:%S")?;
    let start_dt = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
    sqlx::query(
        "INSERT INTO tasks (name, cmd, schedule_type, interval_minutes, start_time) VALUES (?, ?, 'interval', ?, ?)"
    )
    .bind(name)
    .bind(cmd)
    .bind(interval)
    .bind(&start_dt.to_rfc3339())
    .execute(db)
    .await?;
    println!("Интервальная задача '{}' добавлена.", name);
    Ok(())
}

async fn add_cron(
    db: &SqlitePool,
    name: &str,
    expr: &str,
    cmd: &str,
) -> Result<()> {
    // Проверка корректности выражения
    let _schedule = Schedule::from_str(expr)?;
    sqlx::query(
        "INSERT INTO tasks (name, cmd, schedule_type, cron_expr) VALUES (?, ?, 'cron', ?)"
    )
    .bind(name)
    .bind(cmd)
    .bind(expr)
    .execute(db)
    .await?;
    println!("Cron-задача '{}' добавлена.", name);
    Ok(())
}

async fn list_tasks(db: &SqlitePool) -> Result<()> {
    let tasks: Vec<Task> = sqlx::query_as("SELECT * FROM tasks").fetch_all(db).await?;
    for t in tasks {
        println!(
            "name={} | type={} | cmd={} | interval={:?} | start={:?} | cron={:?}",
            t.name, t.schedule_type, t.cmd,
            t.interval_minutes, t.start_time, t.cron_expr
        );
    }
    Ok(())
}

async fn remove_task(db: &SqlitePool, name: &str) -> Result<()> {
    sqlx::query("DELETE FROM tasks WHERE name = ?").bind(name).execute(db).await?;
    println!("Задача '{}' удалена.", name);
    Ok(())
}

async fn run_scheduler(db: &SqlitePool) -> Result<()> {
    println!("Запуск планировщика...");
    loop {
        let now = Utc::now();
        let tasks: Vec<Task> = sqlx::query_as("SELECT * FROM tasks").fetch_all(db).await?;
        for t in tasks {
            if should_run(&t, &now)? {
                let name = t.name.clone();
                let cmd = t.cmd.clone();
                let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
                tokio::spawn(async move {
                    #[cfg(target_family = "unix")]
                    {
                        let mut binder = Command::new("sh");
                        let child = binder.arg("-c").arg(&cmd);
                        if let Err(e) = child.spawn() {
                            eprintln!("Ошибка {}: {}", &cmd, e);
                        }
                    }
                    #[cfg(target_family = "windows")]
                    {
                        let mut binder = Command::new("cmd");
                        let child = binder.arg("/C").arg(&cmd);
                        if let Err(e) = child.spawn() {
                            eprintln!("Ошибка {}: {}", &cmd, e);
                        }
                    }
                    // Логируем
                    println!("[{} {}] запуск: {}", timestamp, name, cmd);
                });
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}

fn should_run(task: &Task, now: &DateTime<Utc>) -> Result<bool> {
    match task.schedule_type.as_str() {
        "interval" => {
            let iv = task.interval_minutes.unwrap();
            let st: DateTime<Utc> = task.start_time.as_ref().unwrap().parse()?;
            if now >= &st {
                let diff = now.signed_duration_since(st);
                let interval_secs = iv * 60;
                return Ok(diff.num_seconds() % interval_secs == 0);
            }
            Ok(false)
        }
        "cron" => {
            let expr = task.cron_expr.as_ref().unwrap();
            let schedule = Schedule::from_str(expr)?;
            let prev = *now - ChronoDuration::seconds(1);
            if let Some(next) = schedule.after(&prev).next() {
                return Ok(next.timestamp() == now.timestamp());
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}