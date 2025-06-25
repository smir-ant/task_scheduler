# Асинхронный кроссплатформенный планировщик задач

Позволяет менеджить (создавать, просматривать удалять) и выполнять CLI команды по расписанию в двух вариантах:
- [cron] каждый час в будние дни
- [interval] каждые 5 минут с 2025-05-20 09:59:30

Задачи хранятся в sqlite.

Примеры использования здесь <code>./task_scheduler --help</code>

### Начать работу

0. У тебя до сих пор не установлен rust?
1. <code>cargo +nightly build --release --artifact-dir ./ -Z unstable-options</code> - бинарь появится в папке с проектом + nightly для оптимизации
2. <code>./task_scheduler init</code> - инициализация бд (sqlite tasks.db)
3. <code>./task_scheduler</code> для вывода всех возможностей
4. Пользуйся помощью: 
    - <code>./task_scheduler --help</code>
    - <code>./task_scheduler remove --help</code>
    - <code>./task_scheduler add-cron --help</code>
    - и т.д.



##### Про оптимизацию и зачем nightly
У меня с 4.7мб бинарник уменьшился до 2.8, ну или 1.7 если сжатие > скорость (<code>opt-level = "z"</code>)
