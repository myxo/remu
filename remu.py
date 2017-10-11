import telebot
import logging
import time
import threading
import argparse
# from datetime import date, time, datetime, timedelta

import libremu_backend as engine

logging.basicConfig(filename='log.txt', format='[%(asctime)s] [%(levelname)s]  %(message)s', level=logging.INFO)

f = open('token.id', 'r')
token = f.read()
f.close()
bot = telebot.TeleBot(token)
rep_id_list = [] # all aroun this var is unsafe now TODO
rep_event_dict = {}

# @bot.message_handler(content_types=['document', 'audio'])
# def handle_docs_audio(message):
#     file_info = bot.get_file(message.document.file_id)
#     file = bot.download_file(file_info.file_path)
#     with open(message.document.file_name, 'wb') as f:
#         f.write(file)


@bot.message_handler(commands=['start'])
def handle_list(message):
    engine.add_user(message.from_user.id, message.from_user.username, message.chat.id, -3)
    bot.send_message(message.chat.id, 'Hello! ^_^')


@bot.message_handler(commands=['list'])
def handle_list(message):
    l = engine.get_active_events()
    text = '\n'.join(l) if l else 'No current active event'
    bot.send_message(message.chat.id, text)


@bot.message_handler(commands=['delete_rep'])
def handle_rep_list(message):
    l = engine.get_rep_events()
    global rep_id_list
    global rep_event_dict

    if not l:
        bot.send_message(message.chat.id, 'No current rep event')
        return

    [text_list, rep_id_list] = list(zip(*l))
    rep_event_dict[message.chat.id] = rep_id_list

    text = [ str(i+1) + ") " + key for i, key in enumerate(text_list)]
    markup = telebot.types.ForceReply(selective=False)
    bot.send_message(message.chat.id, text, reply_markup=markup)
    bot.register_next_step_handler(message, delete_rep_event)


@bot.message_handler(content_types=["text"])
def send_to_engine(message):
    text = engine.handle_text_message(message.from_user.id, message.text)
    bot.send_message(message.chat.id, text)


@bot.callback_query_handler(func=lambda call: True)
def callback_inline(call):
    if call.message:
        if call.data != "Ok":
            handle_user_message(call.data + " " + call.message.text)
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
    # chat_id = message.chat.id
    event_id = message.text
    if not event_id.isdigit():
        msg = bot.reply_to(message, 'You should write number')
        bot.register_next_step_handler(message, delete_rep_event)
        return
    del_id = rep_event_dict[message.chat.id][int(event_id)-1]
    engine.del_rep_event(del_id)

if __name__ == '__main__':
    # engine.register_action_callback( lambda text: bot.send_message(chat_id, text))
    parser = argparse.ArgumentParser()
    parser.add_argument("-v", "--verbose", help="show log lines in stdout",
                    action="store_true")
    args = parser.parse_args()
    verbose = False
    if args.verbose:
        verbose = True

    # read_chat_id()

    engine.initialize(verbose)
    engine.register_callback(callback)
    engine.run()
    try:
        bot.polling()
        logging.error("I am down =(")
    except:
        logging.error("Try to polling again")
        bot.polling()
        logging.error("After pooling again")
        send_message("I've been down and now should work fine")

    engine.stop()
