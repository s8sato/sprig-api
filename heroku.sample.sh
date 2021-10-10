#!/bin/bash
set -e
readonly APP_NAME=sprig-demo-api

readonly ACCESS_CONTROL_ALLOW_ORIGIN=https://s8sato.github.io
readonly CMD_HELP_DIR=/usr/local/share/help
# DATABASE_URL: Given by Heroku Postgres
readonly EMAIL_API=SparkPost # or SendGrid
readonly EMAIL_API_KEY=####
readonly INDENT='    '
readonly IS_CROSS_ORIGIN=true
# PORT: Given by Heroku
readonly SECRET_KEY=####
readonly SENDER_NAME='Sprig'
readonly SENDING_EMAIL_ADDRESS=####

heroku create $APP_NAME
heroku stack:set container

heroku addons:create heroku-postgresql:hobby-dev

heroku config:set ACCESS_CONTROL_ALLOW_ORIGIN=$ACCESS_CONTROL_ALLOW_ORIGIN
heroku config:set CMD_HELP_DIR=$CMD_HELP_DIR
# DATABASE_URL: Given by Heroku Postgres
heroku config:set EMAIL_API=$EMAIL_API
heroku config:set EMAIL_API_KEY=$EMAIL_API_KEY
heroku config:set INDENT="$INDENT"
heroku config:set IS_CROSS_ORIGIN=$IS_CROSS_ORIGIN
# PORT: Given by Heroku
heroku config:set SECRET_KEY=$SECRET_KEY
heroku config:set SENDER_NAME="$SENDER_NAME"
heroku config:set SENDING_EMAIL_ADDRESS=$SENDING_EMAIL_ADDRESS

git push heroku <branchname>:main
