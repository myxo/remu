main_help_message_ru = r"""
Remu - бот для напоминания о ваших событиях. Событие - это просто текст, который Remu напишет вам в заданное время.
В данный момент есть 2 типа событий: единичные и повторяющиеся. 

*Единичные события* можно установить двумя способами: указав *ВО* сколько или *ЧЕРЕЗ* сколько событие должно произойти. Как это указать? Проще понять на примерах.

Пример для *во* сколько:
```
10-11 at 12.30 ололо - 10 ноября в 12.30
10 at 11 траляля - 10 числа этого месяца в 11.00
в 9.35 трюлюлю  - сегодня в 9.35
в 22 ohaha - сегодня в 10 вечера
```
Пример для *через* сколько:
```
1d2h3m4s text1  - 1 день, 2 часа, 3 мин., 4 сек.
1д2ч3м4с текст2 - тоже, но на русском
2ч30м text3     - 2 часа 30 мин.
1c text4        - 1 секунда
```
Ну и более формально, синтаксис для первого случая: 
<день>-<месяц> [at|в] <час>.<минута> <ваш текст события>. 
Обязательным здесь является частица at(или в), час и текст события.

Синтаксис для второго случая
<>d<>h<>m<>s <текст события>
где в <> должно стоять число дней(d), часов(h), минут(m), секунд(s). Русскими буквами (д, ч, м, с) тоже пойдет. Достаточно чтобы было заполнено хотя бы одно поле.

*Повторяющиеся события* - это события у которых есть начальное время и время, через которое они должны повторятся. Искользуется синтаксис единичных событий. Пример:
```
rep 23-12 11.30 7d позвони маме
```
Данное событие будет каждую неделю в 11.30, начиная с 23 декабря напоминать вам позвонить маме. 

"""
