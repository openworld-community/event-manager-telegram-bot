[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/smartlike-org/smartlike/LICENSE)


# Event management bot for Telegram

The bot can help with basic event management tasks like sign up, cancel, remind, manage waiting lists, etc.

## Install

-   Register a bot, configure access token, set administrators in the config file, build and start.
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
