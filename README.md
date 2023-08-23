[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/smartlike-org/smartlike/LICENSE)

# Peredelano Updates

Так как нам необхдимо управлять мероприятиями, мы решили взять готовый бот и переделать его под наши нужды. 
Тестово развёрнутый бот: https://t.me/peredelanoconf_em_test_bot (доступен только клиенсткий функционал)

## Что бот умеет делать сейчас
### Админский функционал
Админ указанный в конфиге бота может создавать, удалять, изменять мероприятия, банить пользователей, назначать и убирать менеджера мероприятия, удалять бронирования, рассылать сообщения забронировавшим пользователям
Мероприятия создаются с помощью отправки JSON-а боту от админа:
```
{
  "name": "Peredelanoconf Yerevan TEST EVENT FOR BOT", # имя мероприятия
  "link": "https://t.me/peredelanoconfyerevan/579", # ссылка на мероприятие
  "Start": "2023-06-17 15:00 +00:00", # время начала мероприятия
  "remind": "2023-05-28 15:00 +00:00", # время отправки напоминания по мероприятию
  "max_adults": 100, # максимальное количество мест для взрослых, которое может быть забронировано, при превышение количества, люди будут помещаться в лист ожидания
  "max_children": 100, # максимальное количество мест для детей, которое может быть забронировано, при превышение количества, люди будут помещаться в лист ожидания
                        # Если и там, и там 0 0 - то мероприятие является просто анонсом, и на переходе по кнопке в списке мероприятий пользователь переходе по ссылке link
  "max_adults_per_reservation": 4, # максимальное количество мест для взрослых, которое может быть забронировано одним пользователем
  "max_children_per_reservation": 5, # максимальное количество мест для детей, которое может быть забронировано одним пользователем
  "adult_ticket_price": 120, # ценник бронирования места для взрослых, может быть 0, если мероприятие бесплатно для взрослых
  "children_ticket_price": 60, # ценник бронирования места для детей, может быть 0, если мероприятие бесплатно для детей
  "currency": "USD" # валюта оплаты
}
```
### Пользователський функционал
Пользователь пишет боту, в ответ получает список предстоящих мероприятий:

![image](https://github.com/openworld-community/event-manager-telegram-bot/assets/40787135/12a32adc-0177-421d-a4bf-856d6ff39693)

При выборе мероприятия будет показано сообщение бронирования:

![image](https://github.com/openworld-community/event-manager-telegram-bot/assets/40787135/80de2d25-0365-4559-8488-c13dcf9bc737)

В котором можно выбрать сколько нужно забронировать места и после перейти к оплате

![image](https://github.com/openworld-community/event-manager-telegram-bot/assets/40787135/9db30003-6422-4bbc-ac93-5c801f7c1b30)

После оплаты при выборе видно будет на какое мероприятия пользователь забронировал и оплатил билет:
![image](https://github.com/openworld-community/event-manager-telegram-bot/assets/40787135/8ba29ad5-7303-4127-be0e-9d1a70eb96cd)
![image](https://github.com/openworld-community/event-manager-telegram-bot/assets/40787135/0d5109f7-f714-4102-bd93-45b9572ec927)

### Функционал менеджера мероприятий - не протестировано

## Наш TODO

### Добавить возможность добавления новых ивентов через WebView админку
### Переехать в Постгресс
### Сделать миграцию БД корректную при измениях в БД
### Добавить оплату в крипте
### Добавить API для подтягивания данных для аналитики и интеграции с CRM
### Добавить фронт для клиентского интерфейса
### Поправить CI/CD по красоте
### Добавить возможность добавления разных организаций в одного бота
### Пофиксить баг с часовыми поясами

# Event management bot for Telegram

The bot can help with basic event management tasks like sign up, cancel, remind, manage waiting lists, etc.

## Install

-   Register a bot, configure access token, set administrators in the config file, build and Start.
-   Check /help for controls.
-   Invite your audience to sign up using links returned on event creation.

## To do

-   Internationalization. The initial language choice was made to help a european non-profit organize events for children and their mothers fleeing the war in Ukraine. ![flag](https://smartlike.org/favicons/ukraine.svg)

# Change log
**v0.1.0**
added custom currency setting for price of the event

**v0.0.3**
added mounting volume for sqlite3 database from docker for storing events and users

**v0.0.2** 
added dockerfile and docker-compose for running application, fixed message when event added
added playbook for deploying application to the server and preparing it for running
added release workflow for creating release and deploying it to the server
added registry publication workflow for publishing docker image to the github registry

**v0.0.1**
added basic functionality for creating events and signing up for them
# Infrastructure
Need host where will be deployed docker containers with the application
Playbooks works only with Ubuntu 20.04
# Needed environment variables
## Repository secrets
DEPLOY_SSH_KEY - private key for ssh access to the target server

DEPLOY_SSH_USER - username for ssh access to the target server

DEPLOY_USER_PASSWORD - password for the user on the target server

DOCKER_HOST_ADDRESS - address of the docker host where will be deployed

PAT_SECRET - personal access token for github actions

## Github actions secrets
### prerelease env
PAYMENT_PROVIDER_TOKEN - token for payment provider for prerelease env

TELEGRAM_BOT_TOKEN - token for telegram bot for prerelease env
### main env
PAYMENT_PROVIDER_TOKEN - token for payment provider for main env

TELEGRAM_BOT_TOKEN - token for telegram bot for main env
