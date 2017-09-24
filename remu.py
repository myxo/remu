import telebot
import logging
import time
import threading
# from datetime import date, time, datetime, timedelta

import libtelegram_rust_backend as engine

logging.basicConfig(filename='log.txt', format='[%(asctime)s] [%(levelname)s]  %(message)s', level=logging.INFO)

f = open('token.id', 'r')
token = f.read()
bot = telebot.TeleBot(token)
chat_id = 0

# @bot.message_handler(content_types=['document', 'audio'])
# def handle_docs_audio(message):
#     file_info = bot.get_file(message.document.file_id)
#     file = bot.download_file(file_info.file_path)
#     with open(message.document.file_name, 'wb') as f:
#         f.write(file)

@bot.message_handler(commands=['list', 'help'])
def handle_list(message):
    l = engine.get_active_events()
    text = '\n'.join(l)
    bot.send_message(chat_id, text)

@bot.message_handler(content_types=["text"])
def send_to_engine(message):
    global chat_id
    # if chat_id != message.chat.id:
        # logging.info('chat_id changed!')
    chat_id = message.chat.id
    handle_user_message(message.text)


@bot.callback_query_handler(func=lambda call: True)
def callback_inline(call):
    if call.message:
        if call.data != "Ok":
            handle_user_message(call.data + " " + call.message.text)
        # delete keys
        bot.edit_message_text(chat_id=call.message.chat.id, message_id=call.message.message_id, text=call.message.text)


def handle_user_message(message_text):
    text = engine.handle_text_message(message_text)
    bot.send_message(chat_id, text)


# temporal solution
def run_check_loop():
    while True:
        text = engine.check_for_message()
        if text != "":
            send_message(text)
        time.sleep(0.5)


def send_message(message_text):
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

if __name__ == '__main__':
    # engine.register_action_callback( lambda text: bot.send_message(chat_id, text))
    engine.initialize(True)
    # engine.run()
    threading.Thread(target=run_check_loop).start()
    bot.polling(none_stop=True)

    engine.stop()
