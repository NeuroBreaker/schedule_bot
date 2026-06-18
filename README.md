# Телеграм бот для просмотра расписания СамГУ

---
## Для Разработчиков
```bash
git clone https://github.com/NeuroBreaker/schedule_bot.git
cd schedule_bot
```
**Создайте файл _.env_ внутри директории и сохраните в него строку со своим токеном**
```
TELOXIDE_TOKEN="12345678:affjlk..."  
```
> [!NOTE]
> Токен создаётся у официального телеграмм бота:  
> @BotFather

> [!TIP]
> Для логгирования необходимо добавить ещё одну строку в .env  
> RUST_LOG=info

**Запуск**
```bash
docker-compose up --build
```
