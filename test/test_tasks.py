# Это скрипт для теста планировщика (планировщиком можно запускать этот скрипт, с аргументами или без)
# ./task_scheduler add-cron --name python5s --expr "1/5 * * * * *" --cmd "python3 ./test/test_tasks.py"
# ну или абсолютный путь или add-interval
import sys
import logging
import os

# Получаем директорию, где находится скрипт
script_dir = os.path.dirname(os.path.abspath(__file__))
log_file = os.path.join(script_dir, 'task.log')

# Настройка логирования
logging.basicConfig(
    filename=log_file,
    filemode='a',  # Дописывать в файл
    level=logging.INFO,
    format='%(asctime)s %(levelname)s:%(message)s',
    force=True
)

def main():
    args = sys.argv[1:]
    if args:
        logging.info(f"Скрипт запущен с аргументами: {args}")
        print(f"Аргументы: {args}")
    else:
        logging.info("Скрипт запущен без аргументов")
        print("Нет аргументов")

if __name__ == '__main__':
    logging.info("Запуск питона")
    main()