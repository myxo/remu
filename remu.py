import telebot
import logging
import time
import threading
import argparse
import datetime
import text_data as text

import libremu_backend as engine
from telegramcalendar import create_calendar

logging.basicConfig(filename='log.txt', format='[%(asctime)s] [%(levelname)s]  %(message)s', level=logging.INFO)

f = open('token.id', 'r')
token = f.read()
f.close()
bot = telebot.TeleBot(token)
fsm = {}

from enum import Enum
class BotState(Enum):
    WAIT                = 0
    REP_DELETE_CHOOSE   = 1
    AT_CALENDAR         = 2
    AT_TIME_TEXT        = 3

class FSMData:
    state = BotState.WAIT
    data = {}

    def reset(self):
        self.state = BotState.WAIT
        self.data = {}


@bot.message_handler(commands=['start'])
def handle_start(message):
    engine.add_user(message.from_user.id, message.from_user.username, message.chat.id, -3)
    bot.send_message(message.chat.id, 'Hello! ^_^\nType /help')


@bot.message_handler(commands=['help'])
def handle_help(message):
    bot.send_message(message.chat.id, text.main_help_message_ru, parse_mode='Markdown')


@bot.message_handler(commands=['list'])
def handle_list(message):
    l = engine.get_active_events(message.from_user.id)
    text = '\n'.join(l) if l else 'No current active event'
    bot.send_message(message.chat.id, text)


def handle_delete_rep(message):
    global fsm
    uid = message.chat.id
    fsm[uid].state = BotState.WAIT
    rep_event_list = engine.get_rep_events(uid)

    if not rep_event_list:
        bot.send_message(message.chat.id, 'No current rep event')
        return

    [text_list, rep_id_list] = list(zip(*rep_event_list))
    fsm[uid].state = BotState.REP_DELETE_CHOOSE
    fsm[uid].data = rep_id_list

    header = "Here is yout rep events list. Choose witch to delete:\n"
    list_str = '\n'.join([ str(i+1) + ") " + key for i, key in enumerate(text_list)])
    bot.send_message(uid, header + list_str)


@bot.message_handler(content_types=["text"])
def send_to_engine(message):
    input_text = message.text
    id = message.chat.id

    if fsm[id].state == BotState.WAIT:
        if input_text == "/delete_rep":
            handle_delete_rep(message)
        else:
            text = engine.handle_text_message(message.chat.id, input_text)
            bot.send_message(message.chat.id, text)
    
    elif fsm[id].state == BotState.REP_DELETE_CHOOSE:
        delete_rep_event(message)
    
    else:
        log.error("Unknown bot state: uid = " + str(id) + " state = " + str(fsm[id].state))


@bot.callback_query_handler(func=lambda call: True)
def callback_inline(call):
    if call.message:
        if call.data != "Ok":
            call.message.text = call.data + " " + call.message.text
            send_to_engine(call.message)
        # delete keys
        bot.edit_message_text(chat_id=call.message.chat.id, message_id=call.message.message_id, text=call.message.text)


def send_message(message_text, chat_id):
    keyboard = telebot.types.InlineKeyboardMarkup()
    callback_button_5m = telebot.types.InlineKeyboardButton(text="5m", callback_data="5m")
    callback_button_30m = telebot.types.InlineKeyboardButton(text="30m", callback_data="30m")
    callback_button_1h = telebot.types.InlineKeyboardButton(text="1h", callback_data="1h")
    keyboard.add(callback_button_5m, callback_button_30m, callback_button_1h)
    callback_button_3h = telebot.types.InlineKeyboardButton(text="3h", callback_data="3h")
    callback_button_1d = telebot.types.InlineKeyboardButton(text="1d", callback_data="1d")
    callback_button_ok = telebot.types.InlineKeyboardButton(text="Ok", callback_data="Ok")
    keyboard.add(callback_button_3h, callback_button_1d, callback_button_ok)
    bot.send_message(chat_id, message_text, reply_markup=keyboard)


def callback(text, chat_id):
    send_message(text, chat_id)


def delete_rep_event(message):
    event_id_str = message.text
    if not event_id_str.isdigit():
        msg = bot.reply_to(message, 'You should write number')
        bot.register_next_step_handler(message, delete_rep_event)
        return

    event_id = int(event_id_str)-1
    if event_id >= 0 and event_id < len(fsm[message.chat.id].data):
        del_id = fsm[message.chat.id].data[event_id]
        engine.del_rep_event(del_id)
        fsm[message.chat.id].reset()
        bot.send_message(message.chat.id, "Done.")
    else:
        bot.send_message(message.chat.id, "Number is out of limit. Try againg.")


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("-v", "--verbose", help="show log lines in stdout",
                    action="store_true")
    args = parser.parse_args()
    verbose = False
    if args.verbose:
        verbose = True

    engine.initialize(verbose)
    engine.register_callback(callback)
    user_chat_id_list = engine.get_user_chat_id_all()
    for chat_id in user_chat_id_list:
        fsm[chat_id] = FSMData()
        fsm[chat_id].state = BotState.WAIT

    engine.run()

    while True:
        try:
            bot.polling()
        except:
            logging.error("I am down =(")
        # break

    engine.stop()
