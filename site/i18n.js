/*
 * 配布ページの訳。**英語は `index.html` に直接書いてあります。**
 * ここに無い言語・無いキーは、そのまま英語が出ます —— **虫食いになりません。**
 *
 * 言語の並びはアプリの `apps/desktop/src/lib/i18n/locales.ts` と同じ 11 種。
 * **アプリで出せる言語と、配布ページで出せる言語を食い違わせない。**
 *
 * 道具の 1 行説明（`tools.json`）は**訳しません。**
 * あれは AI が実際に読む文字列そのもので、訳すと
 * 「載っているもの」と「届くもの」が別になります。
 */

/** 道具の束の見出し（英語）。訳は下の表の `tools.*`。 */
window.SSHBOARD_GROUPS = {
  connect: 'Connecting and choosing where things go',
  look: 'Looking at the server',
  write: 'Writing (inside your fence only)',
  cmds: 'Commands you allowed',
  console: 'The shared terminal',
  screen: "The human's screen",
  itself: 'About sshboard itself',
};

window.SSHBOARD_I18N = {
  ru: {
    'lede': 'Вы и ваш ИИ-агент — на одной SSH-сессии.',
    'tag.alpha': 'Альфа',
    'tag.mac': 'macOS (Apple Silicon)',
    'tag.win': 'Windows x64',
    'tag.langs': '11 языков',
    'intro':
      'Большинство инструментов для агентов дают модели дыру <code>run_command(cmd)</code> и позволяют ей работать <b>там, куда никто не смотрит</b>. sshboard делает наоборот: агент и вы держите <b>одно и то же SSH-соединение</b>, и всё, что он делает, попадает на экран, перед которым вы сидите. <b>Второй, невидимой сессии не существует.</b>',
    'h.download': 'Скачать',
    'dl.button': 'Скачать',
    'dl.mac.sub': 'Apple Silicon',
    'dl.win.sub': 'x64 · установщик (NSIS)',
    'dl.msi.title': 'Windows (MSI)',
    'dl.msi.sub': 'Если у вас разворачивают через MSI',
    'dl.src.title': 'Исходный код',
    'unsigned':
      '<b>Подписи нет.</b> В первый раз вас остановят SmartScreen в Windows и Gatekeeper в macOS. В Windows: <i>Подробнее → Выполнить в любом случае</i>. В macOS: <i>правый клик по приложению → Открыть</i>. <b>Это инструмент, работающий с ключами.</b> Мы понимаем, что первое, чему он вас учит, — отмахнуться от предупреждения; <b>подпись появится тогда, когда её отсутствие кому-то реально навредит</b>, и <a href="https://github.com/meta-taro/sshboard/issues">написанный об этом issue</a> — это то, что решит вопрос.',
    'h.install.mac': 'Установка в macOS',
    'install.mac':
      'Сборка для macOS поставляется как <code>.app.tar.gz</code>, а не <code>.dmg</code>, потому что <b>этот же файл использует автообновление</b>. Распакуйте и перенесите:',
    'install.mac.open': 'Затем откройте из Finder через <b>правый клик → Открыть</b> — только в первый раз.',
    'h.install.win': 'Установка в Windows',
    'install.win':
      'Запустите <code>-setup.exe</code>. Нужен <b>WebView2</b>; в Windows 11 и свежих 10 он уже есть, иначе установщик его скачает — то есть <b>для первой установки нужна сеть.</b>',
    'h.ui': 'Что вы видите',
    'ui.intro':
      'Одно окно, пять вкладок. <b>Снимков экрана на этой странице пока нет.</b> На любом полезном снимке этого продукта видно чьё-то имя хоста, имя пользователя и пути — поэтому те, что здесь появятся, будут <b>сняты на вымышленной конфигурации.</b> А пока — вот его форма.',
    'tab.connections': 'Соединения',
    'tab.files': 'Файлы',
    'tab.console': 'Консоль',
    'tab.band': 'Действия',
    'tab.diag': 'Журнал',
    'ui.left.title': 'Слева',
    'ui.left': 'Серверы, которые вы добавили, по строке на каждый. Открытый становится вкладкой наверху рабочей области.',
    'ui.right.title': 'Справа',
    'ui.right': 'То, для чего эта вкладка — список каталога, живой терминал или одна из двух записей ниже.',
    'ui.th.tab': 'Вкладка',
    'ui.th.what': 'Что в ней',
    'ui.t.connections':
      'Добавленные серверы и кнопка, которая открывает один из них. <b>Учётные данные живут в хранилище учётных данных ОС и в ssh-agent, но не здесь.</b>',
    'ui.t.files':
      'Две панели: сервер с одной стороны, ваша машина с другой. Загрузить, скачать, создать каталог. <b>Агент огорожен каталогами, которые вы перечислили; вы — нет.</b>',
    'ui.t.console':
      'Настоящий терминал на том же SSH-соединении — <b>общий с агентом.</b> Пока агент держит его, ваш ввод заблокирован и показана кнопка «Стоп». <b>Он никогда не открывает вторую сессию, которой вы не видите.</b>',
    'ui.t.band':
      '<b>Кто что сделал.</b> По строке на операцию, с пометкой <code>[Human]</code> или <code>[AI]</code>, в порядке событий. <b>Это вкладка, которую читают, чтобы ответить на вопрос «чем вообще занимался агент?»</b>',
    'ui.t.diag':
      '<b>Почему что-то не получилось.</b> Этапы подключения, ошибки и что делать дальше. <b>Вкладка, которую читают, когда не сработало</b>, — и из которой копируют, когда заводят issue.',
    'ui.confusable':
      '<b>«Действия» и «Журнал» — не одно и то же</b>, и названия пока не делают это достаточно очевидным. <b>«Действия» — запись поступков</b>: отвечает на <i>кто что сделал</i>. <b>«Журнал» — запись диагностики</b>: отвечает на <i>почему не получилось</i>. Хотите узнать, что запускал агент, — вам в «Действия». Хотите понять, почему не открывалось соединение, — в «Журнал».',
    'h.howto': 'Как этим пользоваться',
    'howto.1':
      '<b>Добавьте сервер.</b> Вкладка «Соединения» → добавить. Вы задаёте id, имя, хост и пользователя. <b>sshboard не хранит ни парольную фразу, ни пароль</b> — они уходят в хранилище учётных данных ОС, либо вы оставляете это ssh-agent.',
    'howto.2':
      '<b>Откройте его.</b> Он станет вкладкой. Открытыми могут быть сразу несколько, и <b>каждое из них видно.</b> Если ключу нужна парольная фраза, <b>вопрос появится на вашем экране</b> — <b>агент не может на него ответить.</b>',
    'howto.3':
      '<b>Работайте в «Файлах» или «Консоли».</b> И то, и другое идёт по этому же соединению. <b>Ничего из того, что делает агент, не происходит на другом.</b>',
    'howto.4':
      '<b>Подключите к нему своего агента</b> (ниже). Попросите его что-нибудь посмотреть. <b>То, что он запускает, появляется в «Действиях» по ходу дела</b>, а вывод — <b>в той же консоли, на которую смотрите вы.</b>',
    'howto.5':
      '<b>То, что меняет состояние, подтверждает человек.</b> Чтение огорожено <b>списком разрешений, который написали вы</b>. Всё, что меняет состояние, <b>в первый раз отклоняется</b> и <b>показывается вам целиком</b> прежде, чем сможет выполниться, — а подтверждение действует <b>на один запуск, пять минут.</b>',
    'howto.empty':
      '<b>Сразу после установки агент не может вообще ничего.</b> Список разрешений на чтение пуст, каталоги для записи пусты, список операций, меняющих состояние, пуст. <b>Это не забытый шаг настройки — так задумано.</b> Вы расширяете ровно настолько, насколько действительно нужно, по одной строке.',
    'h.tools': 'Что может агент',
    'tools.intro':
      '<span id="tool-count"></span> инструментов через MCP. <b>Здесь приведены в точности те описания, которые читает агент</b>, поэтому они на английском. Обратите внимание, чего здесь <b>нет</b>: нет инструмента, принимающего произвольную строку команды, и нет инструмента, скачивающего на вашу машину.',
    'tools.connect': 'Подключиться и выбрать, куда всё уходит',
    'tools.look': 'Смотреть на сервер',
    'tools.write': 'Запись (только внутри вашей ограды)',
    'tools.cmds': 'Команды, которые вы разрешили',
    'tools.console': 'Общий терминал',
    'tools.screen': 'Экран человека',
    'tools.itself': 'О самом sshboard',
    'h.state': 'Где это на самом деле сейчас',
    'state.intro':
      '<b>Возможности на месте. Интерфейс не доделан.</b> sshboard используют против настоящего боевого сервера, и весь видимый список issue вышел именно оттуда — но <b>ни одного прохода по дизайну ещё не было, и это заметно.</b> Если вы ищете вылизанное — пока не сюда. Если вы ищете то, <b>чью модель безопасности можно прочитать от начала до конца и оспорить</b>, — читать уже можно.',
    'state.working': 'Работает',
    'state.notyet': 'Пока нет',
    'state.working.list':
      'Одна общая SSH-сессия (интерфейс + агент)<br />35 инструментов MCP<br />Команды только на чтение из списка разрешений<br />Подтверждаемые операции, меняющие состояние<br /><code>sudo</code> с паролем, который нигде не хранится<br />Общий терминал с кнопкой остановки<br />Автообновление',
    'state.notyet.list':
      'Проход по дизайну интерфейса<br />Снимки экрана на этой странице<br />Подпись кода<br />ssh-agent в Windows (именованный канал / Pageant) — не проверено<br />Сборки для macOS Intel / Linux<br />Тесты, которые отрисовывают сам экран',
    'h.fence': 'Ограда',
    'fence.intro':
      '<b>Это ограничения в коде, а не пожелания в промпте.</b> Они действуют независимо от того, сотрудничает модель или нет.',
    'fence.1':
      '<b>Инструмента <code>run_command(cmd)</code> не существует.</b> Ни отфильтрованного, ни хитрого. <b>Агент не может передать шеллу произвольную строку.</b>',
    'fence.2':
      '<b>Команды только на чтение берутся из списка, который написали вы.</b> <code>readonly.toml</code> <b>изначально пуст</b>, так что «из коробки» агент <b>не может выполнить ни одной команды.</b>',
    'fence.3':
      '<b>Запись огорожена каталогами, которые вы перечислили</b>, для каждого соединения. Этот список <b>тоже изначально пуст.</b> <b>Только загрузка и создание каталога</b> — ни удаления, ни переименования, ни перемещения, ни смены прав, ни перезапуска служб, ни установки пакетов.',
    'fence.4':
      '<b>Инструмента «скачать ко мне на машину» нет.</b> Ограда <b>не защищает ваш ноутбук ни на сантиметр</b>, поэтому двери туда агент не получает.',
    'fence.5':
      '<b>Операции, меняющие состояние, требуют, чтобы человек нажал</b>, и первая попытка всегда отклоняется, чтобы экран мог <b>показать в точности, что будет запущено.</b> Подтверждения <b>истекают через 5 минут и покрывают один запуск.</b>',
    'fence.6':
      '<b>Ключи остаются в хранилище учётных данных ОС и в ssh-agent.</b> sshboard <b>не реализует собственное хранилище ключей</b>, а инструмент, перечисляющий соединения, возвращает <b>идентификаторы, но никогда не учётные данные.</b>',
    'h.agent': 'Подключите своего агента',
    'agent.intro':
      'Направьте агента на запущенное приложение. В этом виде sshboard стартует <b>как ретранслятор без окна</b>, поэтому <b>токен ни разу не записывается на диск</b> — ни в <code>~/.claude.json</code>, ни в историю вашей оболочки.',
    'agent.note':
      'Ретранслятор не держит движок и <b>не открывает собственных соединений</b> — их открывает только приложение, и в этом весь смысл. <b>Это одноразовый шаг</b>: порт зафиксирован на <code>22022</code>, токен переиспользуется, так что перезапуск не заставит регистрировать заново. Если порт занят, он <b>не переезжает молча на другой</b>, а говорит об этом на экране. (Есть и обычный HTTP-вариант, но он <b>оставляет токен в файле</b> — в окне есть кнопка, которая его копирует.)',
    'agent.about':
      'Сначала попросите его вызвать <code>about_sshboard</code> — этот инструмент существует, чтобы агент сам выяснил, что это такое, что ему можно и нельзя и что изменилось в каждой версии, <b>без того чтобы вы вставляли ему вводную.</b>',
    'h.never': 'Чем это не станет',
    'never.1': 'Способом гонять агента против сервера, <b>пока никто не смотрит.</b>',
    'never.2': 'Местом, где <b>хранятся</b> ваши ключи или парольные фразы.',
    'never.3': 'Инструментом, который открывает <b>вторую SSH-сессию, невидимую для вас.</b>',
    'f.source': 'Исходный код',
    'f.releases': 'Релизы',
    'f.issues': 'Issues',
  },

  it: {
    'lede': 'Tu e il tuo agente IA, sulla stessa sessione SSH.',
    'tag.alpha': 'Alfa',
    'tag.mac': 'macOS (Apple Silicon)',
    'tag.win': 'Windows x64',
    'tag.langs': '11 lingue',
    'intro':
      "La maggior parte degli strumenti per agenti dà al modello un buco <code>run_command(cmd)</code> e lo lascia lavorare <b>dove nessuno sta guardando</b>. sshboard fa il contrario: l'agente e tu tenete <b>la stessa connessione SSH</b>, e tutto quello che fa finisce su uno schermo davanti al quale sei seduto. <b>Non esiste una seconda sessione invisibile.</b>",
    'h.download': 'Scarica',
    'dl.button': 'Scarica',
    'dl.mac.sub': 'Apple Silicon',
    'dl.win.sub': 'x64 · programma di installazione (NSIS)',
    'dl.msi.title': 'Windows (MSI)',
    'dl.msi.sub': 'Se la tua organizzazione distribuisce via MSI',
    'dl.src.title': 'Codice sorgente',
    'unsigned':
      "<b>Non è firmato.</b> La prima volta SmartScreen su Windows e Gatekeeper su macOS ti fermeranno. Su Windows: <i>Ulteriori informazioni → Esegui comunque</i>. Su macOS: <i>clic destro sull'app → Apri</i>. <b>Questo è uno strumento che maneggia chiavi.</b> Sappiamo che la prima cosa che ti insegna è ignorare un avviso — <b>la firma arriverà quando a qualcuno la sua assenza farà davvero male</b>, e <a href=\"https://github.com/meta-taro/sshboard/issues\">scriverlo in un issue</a> è ciò che lo deciderà.",
    'h.install.mac': 'Installazione su macOS',
    'install.mac':
      "La build per macOS arriva come <code>.app.tar.gz</code> invece che come <code>.dmg</code>, perché <b>è lo stesso file che usa l'aggiornamento automatico</b>. Estrai e sposta:",
    'install.mac.open': 'Poi aprila dal Finder con <b>clic destro → Apri</b> la prima volta.',
    'h.install.win': 'Installazione su Windows',
    'install.win':
      "Esegui il <code>-setup.exe</code>. Windows ha bisogno di <b>WebView2</b>; Windows 11 e i 10 recenti ce l'hanno già, altrimenti lo scarica il programma di installazione — quindi <b>la prima installazione richiede una connessione.</b>",
    'h.ui': 'Che cosa stai guardando',
    'ui.intro':
      "Una finestra, cinque schede. <b>Su questa pagina non ci sono ancora schermate.</b> Ogni schermata utile di questo prodotto contiene nome host, nome utente e percorsi di qualcuno — quelle che finiranno qui saranno quindi <b>scattate su una configurazione inventata.</b> Nel frattempo, ecco la sua forma.",
    'tab.connections': 'Connessioni',
    'tab.files': 'File',
    'tab.console': 'Console',
    'tab.band': 'Attività',
    'tab.diag': 'Registro',
    'ui.left.title': 'A sinistra',
    'ui.left': "I server che hai registrato, uno per riga. Aprendone uno diventa una scheda in cima all'area di lavoro.",
    'ui.right.title': 'A destra',
    'ui.right': "Ciò a cui serve quella scheda: un elenco di directory, un terminale vivo, o una delle due registrazioni qui sotto.",
    'ui.th.tab': 'Scheda',
    'ui.th.what': 'Che cosa contiene',
    'ui.t.connections':
      "I server registrati e il pulsante che ne apre uno. <b>Le credenziali stanno nel portachiavi del sistema e in ssh-agent, mai qui.</b>",
    'ui.t.files':
      "Due riquadri: da una parte il server, dall'altra la tua macchina. Caricare, scaricare, creare una directory. <b>L'agente è recintato alle directory che hai elencato; tu no.</b>",
    'ui.t.console':
      "Un vero terminale sulla stessa connessione SSH — <b>condiviso con l'agente.</b> Finché l'agente lo tiene, il tuo input è bloccato ed è visibile un pulsante Ferma. <b>Non apre mai una seconda sessione che tu non veda.</b>",
    'ui.t.band':
      "<b>Chi ha fatto cosa.</b> Una riga per operazione, contrassegnata <code>[Human]</code> o <code>[AI]</code>, nell'ordine in cui è successo. <b>È la scheda che si legge per rispondere a «ma l'agente che cosa ha fatto?»</b>",
    'ui.t.diag':
      "<b>Perché qualcosa è fallito.</b> Le fasi della connessione, gli errori e che cosa fare dopo. <b>La scheda che si legge quando non ha funzionato</b> — e quella da cui si copia quando si apre un issue.",
    'ui.confusable':
      "<b>Attività e Registro non sono la stessa cosa</b>, e i nomi non lo rendono ancora abbastanza evidente. <b>Attività è una registrazione di azioni</b>: risponde a <i>chi ha fatto cosa</i>. <b>Registro è una registrazione di diagnosi</b>: risponde a <i>perché è fallito</i>. Se vuoi sapere che cosa ha eseguito l'agente, ti serve Attività. Se vuoi sapere perché una connessione non si apriva, ti serve Registro.",
    'h.howto': 'Come si usa',
    'howto.1':
      "<b>Registra un server.</b> Scheda Connessioni → aggiungine uno. Gli dai un id, un nome, un host e un utente. <b>sshboard non conserva né la passphrase né la password</b> — vanno nel portachiavi del sistema, oppure lasci fare a ssh-agent.",
    'howto.2':
      "<b>Aprilo.</b> Diventa una scheda. Se ne possono tenere aperti diversi insieme, e <b>ognuno di essi è visibile.</b> Se una chiave richiede una passphrase, <b>la domanda compare sul tuo schermo</b> — <b>l'agente non può rispondere.</b>",
    'howto.3':
      "<b>Lavora in File o in Console.</b> Entrambe passano da quella stessa connessione. <b>Niente di ciò che fa l'agente avviene su un'altra.</b>",
    'howto.4':
      "<b>Punta il tuo agente su di essa</b> (qui sotto). Chiedigli di andare a vedere qualcosa. <b>Ciò che esegue compare in Attività mentre accade</b>, e il suo output compare <b>nella stessa console che stai guardando.</b>",
    'howto.5':
      "<b>Ciò che cambia le cose lo approva una persona.</b> La lettura è recintata da un <b>elenco di permessi che hai scritto tu</b>. Tutto ciò che cambia lo stato viene <b>rifiutato la prima volta</b> e <b>mostrato per intero</b> prima di poter essere eseguito — e l'approvazione vale <b>per una esecuzione, per cinque minuti.</b>",
    'howto.empty':
      "<b>Appena installato, l'agente non può fare assolutamente nulla.</b> L'elenco dei permessi in sola lettura è vuoto, le directory scrivibili sono vuote e l'elenco delle operazioni che cambiano lo stato è vuoto. <b>Non è un passaggio di configurazione dimenticato: è il progetto.</b> Lo allarghi solo quanto serve davvero, una riga alla volta.",
    'h.tools': 'Che cosa può fare l’agente',
    'tools.intro':
      "<span id=\"tool-count\"></span> strumenti via MCP. <b>Queste sono esattamente le descrizioni che legge l'agente</b>, perciò restano in inglese. Nota che cosa <b>non</b> c'è: nessuno strumento che accetti una stringa di comando arbitraria e nessuno che scarichi sulla tua macchina.",
    'tools.connect': 'Connettersi e scegliere dove vanno le cose',
    'tools.look': 'Guardare il server',
    'tools.write': 'Scrivere (solo dentro il tuo recinto)',
    'tools.cmds': 'Comandi che hai permesso',
    'tools.console': 'Il terminale condiviso',
    'tools.screen': 'Lo schermo della persona',
    'tools.itself': 'Su sshboard stesso',
    'h.state': 'A che punto è davvero',
    'state.intro':
      "<b>Le funzionalità ci sono. L'interfaccia non è finita.</b> sshboard è usato contro un vero server di produzione, e tutta la lista di issue visibile viene da quell'uso — ma <b>non ha ancora avuto una passata di design, e si vede.</b> Se cerchi qualcosa di rifinito, non è ancora questo. Se cerchi qualcosa <b>il cui modello di sicurezza si possa leggere da cima a fondo e contestare</b>, è già leggibile.",
    'state.working': 'Funziona',
    'state.notyet': 'Non ancora',
    'state.working.list':
      "Una sessione SSH condivisa (interfaccia + agente)<br />35 strumenti MCP<br />Comandi in sola lettura da un elenco di permessi<br />Operazioni che cambiano lo stato, approvate<br /><code>sudo</code> con una password che nessuno memorizza<br />Terminale condiviso con pulsante di arresto<br />Aggiornamento automatico",
    'state.notyet.list':
      "Una passata di design sull'interfaccia<br />Schermate su questa pagina<br />Firma del codice<br />ssh-agent su Windows (named pipe / Pageant) — non verificato<br />Build per macOS Intel / Linux<br />Test che disegnino lo schermo stesso",
    'h.fence': 'Il recinto',
    'fence.intro':
      "<b>Sono vincoli nel codice, non consigli in un prompt.</b> Valgono che il modello collabori o no.",
    'fence.1':
      "<b>Non esiste uno strumento <code>run_command(cmd)</code>.</b> Né filtrato né furbo. <b>Un agente non può consegnare a una shell una stringa arbitraria.</b>",
    'fence.2':
      "<b>I comandi in sola lettura vengono da un elenco che hai scritto tu.</b> <code>readonly.toml</code> <b>parte vuoto</b>, quindi così com'è l'agente <b>non può eseguire un solo comando.</b>",
    'fence.3':
      "<b>La scrittura è recintata alle directory che hai elencato</b>, per ogni connessione. Anche quell'elenco <b>parte vuoto.</b> <b>Solo caricare e creare directory</b> — niente cancellazioni, rinomine, spostamenti, cambi di permessi, riavvii di servizi o installazioni di pacchetti.",
    'fence.4':
      "<b>Non c'è uno strumento per scaricare sulla tua macchina.</b> Il recinto <b>non protegge il tuo portatile di un millimetro</b>, quindi l'agente non riceve una porta verso di esso.",
    'fence.5':
      "<b>Le operazioni che cambiano lo stato richiedono che una persona prema</b>, e il primo tentativo viene sempre rifiutato perché lo schermo possa <b>mostrare esattamente che cosa verrebbe eseguito.</b> Le approvazioni <b>scadono in 5 minuti e coprono una esecuzione.</b>",
    'fence.6':
      "<b>Le chiavi restano nel portachiavi del sistema e in ssh-agent.</b> sshboard <b>non implementa un proprio archivio di chiavi</b>, e lo strumento che elenca le connessioni restituisce <b>identificatori, mai credenziali.</b>",
    'h.agent': 'Collega il tuo agente',
    'agent.intro':
      "Punta il tuo agente sull'app in esecuzione. Questa forma avvia sshboard <b>come relè senza finestra</b>, così <b>il token non viene mai scritto su disco</b> — né in <code>~/.claude.json</code> né nella cronologia della tua shell.",
    'agent.note':
      "Il relè non tiene alcun motore e <b>non apre connessioni proprie</b> — le apre solo l'app, ed è esattamente questo il punto. <b>È un passo una tantum</b>: la porta è fissa a <code>22022</code> e il token viene riusato, quindi riavviare non ti obbliga a registrarlo di nuovo. Se la porta è occupata, <b>non si sposta in silenzio su un'altra</b>: lo dice a schermo. (Esiste anche la forma HTTP semplice, ma quella <b>lascia il token in un file</b> — nella finestra c'è un pulsante che lo copia.)",
    'agent.about':
      "Fagli chiamare prima <code>about_sshboard</code>: quello strumento esiste perché un agente scopra da solo che cos'è questo, che cosa può e non può fare e che cosa è cambiato in ogni versione, <b>senza che tu gli incolli una spiegazione.</b>",
    'h.never': 'Che cosa non diventerà',
    'never.1': "Un modo di far girare un agente contro un server <b>mentre nessuno guarda.</b>",
    'never.2': "Un posto dove le tue chiavi o passphrase <b>vengono conservate.</b>",
    'never.3': "Uno strumento che apre <b>una seconda sessione SSH che tu non puoi vedere.</b>",
    'f.source': 'Codice sorgente',
    'f.releases': 'Release',
    'f.issues': 'Issues',
  },
  'pt-BR': {
    'lede': 'Você e seu agente de IA, na mesma sessão SSH.',
    'tag.alpha': 'Alfa',
    'tag.mac': 'macOS (Apple Silicon)',
    'tag.win': 'Windows x64',
    'tag.langs': '11 idiomas',
    'intro':
      'A maioria das ferramentas para agentes dá ao modelo um buraco <code>run_command(cmd)</code> e o deixa trabalhar <b>onde ninguém está olhando</b>. O sshboard faz o contrário: o agente e você seguram <b>a mesma conexão SSH</b>, e tudo o que ele faz aparece numa tela à sua frente. <b>Não existe uma segunda sessão invisível.</b>',
    'h.download': 'Baixar',
    'dl.button': 'Baixar',
    'dl.mac.sub': 'Apple Silicon',
    'dl.win.sub': 'x64 · instalador (NSIS)',
    'dl.msi.title': 'Windows (MSI)',
    'dl.msi.sub': 'Se sua organização distribui por MSI',
    'dl.src.title': 'Código-fonte',
    'unsigned':
      '<b>Não é assinado.</b> O SmartScreen no Windows e o Gatekeeper no macOS vão barrar você da primeira vez. No Windows: <i>Mais informações → Executar assim mesmo</i>. No macOS: <i>clique com o botão direito no app → Abrir</i>. <b>Esta é uma ferramenta que lida com chaves.</b> Sabemos que a primeira coisa que ela ensina é ignorar um aviso — <b>a assinatura entra quando alguém for realmente prejudicado pela falta dela</b>, e <a href="https://github.com/meta-taro/sshboard/issues">dizer isso num issue</a> é o que decide.',
    'h.install.mac': 'Instalação no macOS',
    'install.mac':
      'A build de macOS vem como <code>.app.tar.gz</code> em vez de <code>.dmg</code>, porque <b>é o mesmo arquivo que a atualização automática consome</b>. Extraia e mova:',
    'install.mac.open': 'Depois abra pelo Finder com <b>clique direito → Abrir</b> na primeira vez.',
    'h.install.win': 'Instalação no Windows',
    'install.win':
      'Execute o <code>-setup.exe</code>. O Windows precisa do <b>WebView2</b>; o Windows 11 e os 10 recentes já têm, e se não tiver o instalador busca — então <b>a primeira instalação precisa de conexão.</b>',
    'h.ui': 'O que você está vendo',
    'ui.intro':
      'Uma janela, cinco abas. <b>Ainda não há capturas de tela nesta página.</b> Qualquer captura útil deste produto tem o nome de host, o usuário e os caminhos de alguém — por isso as que vierem para cá serão <b>tiradas sobre uma configuração fictícia.</b> Até lá, este é o formato dele.',
    'tab.connections': 'Conexões',
    'tab.files': 'Arquivos',
    'tab.console': 'Console',
    'tab.band': 'Atividade',
    'tab.diag': 'Log',
    'ui.left.title': 'Esquerda',
    'ui.left': 'Os servidores que você cadastrou, um por linha. Ao abrir um, ele vira uma aba no topo da área de trabalho.',
    'ui.right.title': 'Direita',
    'ui.right': 'Aquilo para que a aba serve — uma listagem de diretório, um terminal vivo, ou um dos dois registros abaixo.',
    'ui.th.tab': 'Aba',
    'ui.th.what': 'O que ela guarda',
    'ui.t.connections':
      'Os servidores cadastrados e o botão que abre um deles. <b>As credenciais ficam no cofre de credenciais do sistema e no ssh-agent, nunca aqui.</b>',
    'ui.t.files':
      'Dois painéis: o servidor de um lado, sua máquina do outro. Enviar, baixar, criar diretório. <b>O agente está cercado nos diretórios que você listou; você não está.</b>',
    'ui.t.console':
      'Um terminal de verdade sobre essa mesma conexão SSH — <b>compartilhado com o agente.</b> Enquanto o agente o segura, sua digitação fica bloqueada e um botão Parar aparece. <b>Ele nunca abre uma segunda sessão que você não veja.</b>',
    'ui.t.band':
      '<b>Quem fez o quê.</b> Uma linha por operação, marcada <code>[Human]</code> ou <code>[AI]</code>, na ordem em que aconteceu. <b>É a aba que se lê para responder "o que o agente andou fazendo?"</b>',
    'ui.t.diag':
      '<b>Por que algo falhou.</b> As etapas da conexão, os erros e o que fazer a seguir. <b>A aba que se lê quando algo não funcionou</b> — e de onde se copia ao abrir um issue.',
    'ui.confusable':
      '<b>Atividade e Log não são a mesma coisa</b>, e os nomes ainda não deixam isso suficientemente claro. <b>Atividade é um registro de ações</b> — responde <i>quem fez o quê</i>. <b>Log é um registro de diagnóstico</b> — responde <i>por que falhou</i>. Se você quer saber o que o agente executou, quer Atividade. Se quer saber por que uma conexão não abria, quer Log.',
    'h.howto': 'Como se usa',
    'howto.1':
      '<b>Cadastre um servidor.</b> Aba Conexões → adicionar um. Você dá um id, um nome, um host e um usuário. <b>O sshboard não guarda a frase-senha nem a senha</b> — elas vão para o cofre de credenciais do sistema, ou você deixa por conta do ssh-agent.',
    'howto.2':
      '<b>Abra.</b> Ele vira uma aba. Vários podem ficar abertos ao mesmo tempo, e <b>todos eles são visíveis.</b> Se uma chave pedir frase-senha, <b>a pergunta aparece na sua tela</b> — <b>o agente não pode respondê-la.</b>',
    'howto.3':
      '<b>Trabalhe em Arquivos ou no Console.</b> Os dois passam por essa mesma conexão. <b>Nada do que o agente faz acontece em outra.</b>',
    'howto.4':
      '<b>Aponte seu agente para ela</b> (abaixo). Peça que ele olhe alguma coisa. <b>O que ele executa aparece em Atividade conforme acontece</b>, e a saída aparece <b>no mesmo console que você está olhando.</b>',
    'howto.5':
      '<b>O que muda coisas precisa da aprovação de uma pessoa.</b> A leitura é cercada por uma <b>lista de permissões que você escreveu</b>. Tudo que muda o estado é <b>recusado na primeira vez</b> e <b>mostrado inteiro para você</b> antes de poder rodar — e a aprovação vale <b>para uma execução, por cinco minutos.</b>',
    'howto.empty':
      '<b>Recém-instalado, o agente não consegue fazer absolutamente nada.</b> A lista de permissões de leitura está vazia, os diretórios de escrita estão vazios e a lista de operações que mudam o estado está vazia. <b>Não é um passo de configuração esquecido — é o projeto.</b> Você alarga só até onde realmente precisa, uma linha de cada vez.',
    'h.tools': 'O que o agente pode fazer',
    'tools.intro':
      '<span id="tool-count"></span> ferramentas via MCP. <b>Estas são exatamente as descrições que o agente lê</b>, por isso aparecem em inglês. Repare no que <b>não</b> está aqui: nenhuma ferramenta que receba uma string de comando arbitrária, e nenhuma que baixe para a sua máquina.',
    'tools.connect': 'Conectar e escolher para onde as coisas vão',
    'tools.look': 'Olhar o servidor',
    'tools.write': 'Escrever (só dentro da sua cerca)',
    'tools.cmds': 'Comandos que você permitiu',
    'tools.console': 'O terminal compartilhado',
    'tools.screen': 'A tela da pessoa',
    'tools.itself': 'Sobre o próprio sshboard',
    'h.state': 'Onde isto realmente está',
    'state.intro':
      '<b>As funcionalidades estão prontas. A interface não está.</b> O sshboard está sendo usado contra um servidor de produção real, e toda a lista de issues visível saiu desse uso — mas <b>ainda não passou por uma rodada de design, e dá para perceber.</b> Se você procura algo polido, ainda não é. Se procura algo <b>cujo modelo de segurança dê para ler de ponta a ponta e contestar</b>, já dá.',
    'state.working': 'Funciona',
    'state.notyet': 'Ainda não',
    'state.working.list':
      'Uma sessão SSH compartilhada (interface + agente)<br />35 ferramentas MCP<br />Comandos somente leitura a partir de uma lista de permissões<br />Operações de mudança de estado aprovadas<br /><code>sudo</code> com uma senha que ninguém armazena<br />Terminal compartilhado com botão de parar<br />Atualização automática',
    'state.notyet.list':
      'Uma rodada de design na interface<br />Capturas de tela nesta página<br />Assinatura de código<br />ssh-agent no Windows (named pipe / Pageant) — não verificado<br />Builds para macOS Intel / Linux<br />Testes que renderizem a própria tela',
    'h.fence': 'A cerca',
    'fence.intro':
      '<b>São restrições no código, não conselhos num prompt.</b> Elas valem quer o modelo coopere, quer não.',
    'fence.1':
      '<b>Não existe uma ferramenta <code>run_command(cmd)</code>.</b> Nem filtrada, nem esperta. <b>Um agente não tem como entregar uma string arbitrária a um shell.</b>',
    'fence.2':
      '<b>Os comandos somente leitura vêm de uma lista que você escreveu.</b> O <code>readonly.toml</code> <b>começa vazio</b>, então, do jeito que vem, o agente <b>não consegue rodar um único comando.</b>',
    'fence.3':
      '<b>A escrita é cercada nos diretórios que você listou</b>, por conexão. Essa lista <b>também começa vazia.</b> <b>Só enviar e criar diretório</b> — nada de apagar, renomear, mover, mudar permissão, reiniciar serviço ou instalar pacote.',
    'fence.4':
      '<b>Não há ferramenta para baixar para a sua máquina.</b> A cerca <b>não protege o seu notebook em nada</b>, então o agente não ganha uma porta para lá.',
    'fence.5':
      '<b>Operações que mudam o estado precisam de alguém apertando</b>, e a primeira tentativa é sempre recusada para que a tela possa <b>mostrar exatamente o que rodaria.</b> As aprovações <b>expiram em 5 minutos e cobrem uma execução.</b>',
    'fence.6':
      '<b>As chaves ficam no cofre de credenciais do sistema e no ssh-agent.</b> O sshboard <b>não implementa um cofre de chaves</b>, e a ferramenta que lista conexões devolve <b>identificadores, nunca credenciais.</b>',
    'h.agent': 'Conecte seu agente',
    'agent.intro':
      'Aponte seu agente para o app em execução. Esta forma inicia o sshboard <b>como um relé sem janela</b>, de modo que <b>o token nunca é escrito em disco</b> — nem no <code>~/.claude.json</code>, nem no histórico do seu shell.',
    'agent.note':
      'O relé não segura nenhum motor e <b>não abre conexão própria</b> — só o app abre, e é justamente esse o ponto. <b>É um passo de uma vez só</b>: a porta é fixa em <code>22022</code> e o token é reaproveitado, então reiniciar não obriga a cadastrar de novo. Se a porta estiver ocupada, <b>ele não muda de número em silêncio</b>: avisa na tela. (Existe também a forma HTTP simples, mas essa <b>deixa o token num arquivo</b> — a janela tem um botão que copia.)',
    'agent.about':
      'Peça que ele chame <code>about_sshboard</code> primeiro — essa ferramenta existe para que um agente descubra sozinho o que é isto, o que pode e o que não pode fazer, e o que mudou em cada versão, <b>sem que você cole um briefing.</b>',
    'h.never': 'No que isto não vai virar',
    'never.1': 'Um jeito de rodar um agente contra um servidor <b>enquanto ninguém está olhando.</b>',
    'never.2': 'Um lugar onde suas chaves ou frases-senha <b>fiquem guardadas.</b>',
    'never.3': 'Uma ferramenta que abra <b>uma segunda sessão SSH que você não possa ver.</b>',
    'f.source': 'Código-fonte',
    'f.releases': 'Versões',
    'f.issues': 'Issues',
  },
  es: {
    'lede': 'Tú y tu agente de IA, en una sola sesión SSH.',
    'tag.alpha': 'Alfa',
    'tag.mac': 'macOS (Apple Silicon)',
    'tag.win': 'Windows x64',
    'tag.langs': '11 idiomas',
    'intro':
      'La mayoría de las herramientas para agentes le dan al modelo un agujero <code>run_command(cmd)</code> y lo dejan trabajar <b>donde nadie está mirando</b>. sshboard hace lo contrario: el agente y tú sostenéis <b>la misma conexión SSH</b>, y todo lo que hace aparece en una pantalla que tienes delante. <b>No hay una segunda sesión invisible.</b>',
    'h.download': 'Descargar',
    'dl.button': 'Descargar',
    'dl.mac.sub': 'Apple Silicon',
    'dl.win.sub': 'x64 · instalador (NSIS)',
    'dl.msi.title': 'Windows (MSI)',
    'dl.msi.sub': 'Si tu organización despliega por MSI',
    'dl.src.title': 'Código fuente',
    'unsigned':
      '<b>No está firmado.</b> SmartScreen en Windows y Gatekeeper en macOS te detendrán la primera vez. En Windows: <i>Más información → Ejecutar de todas formas</i>. En macOS: <i>clic derecho sobre la app → Abrir</i>. <b>Esta es una herramienta que maneja claves.</b> Sabemos que lo primero que te enseña es a ignorar un aviso — <b>la firma llegará cuando a alguien le perjudique de verdad no tenerla</b>, y <a href="https://github.com/meta-taro/sshboard/issues">decirlo en un issue</a> es lo que lo decidirá.',
    'h.install.mac': 'Instalación en macOS',
    'install.mac':
      'La versión de macOS se distribuye como <code>.app.tar.gz</code> y no como <code>.dmg</code>, porque <b>ese mismo archivo es el que usa la actualización automática</b>. Descomprime y muévelo:',
    'install.mac.open': 'Después ábrelo desde el Finder con <b>clic derecho → Abrir</b> la primera vez.',
    'h.install.win': 'Instalación en Windows',
    'install.win':
      'Ejecuta el <code>-setup.exe</code>. Windows necesita <b>WebView2</b>; Windows 11 y los 10 recientes ya lo traen, y si no el instalador lo descarga — así que <b>la primera instalación necesita conexión a la red.</b>',
    'h.ui': 'Qué estás viendo',
    'ui.intro':
      'Una ventana, cinco pestañas. <b>Todavía no hay capturas de pantalla en esta página.</b> Cualquier captura útil de este producto lleva el nombre de host, el usuario y las rutas de alguien — por eso las que vayan aquí se <b>tomarán sobre una configuración ficticia.</b> Mientras tanto, esta es su forma.',
    'tab.connections': 'Conexiones',
    'tab.files': 'Archivos',
    'tab.console': 'Consola',
    'tab.band': 'Actividad',
    'tab.diag': 'Registro',
    'ui.left.title': 'Izquierda',
    'ui.left': 'Los servidores que has registrado, uno por fila. Al abrir uno se convierte en una pestaña arriba del área de trabajo.',
    'ui.right.title': 'Derecha',
    'ui.right': 'Aquello para lo que sirve esa pestaña: un listado de directorio, una terminal viva, o uno de los dos registros de abajo.',
    'ui.th.tab': 'Pestaña',
    'ui.th.what': 'Qué contiene',
    'ui.t.connections':
      'Los servidores registrados y el botón que abre uno. <b>Las credenciales viven en el almacén de credenciales del sistema y en ssh-agent, nunca aquí.</b>',
    'ui.t.files':
      'Dos paneles: el servidor a un lado, tu máquina al otro. Subir, bajar, crear un directorio. <b>El agente está vallado a los directorios que tú enumeraste; tú no lo estás.</b>',
    'ui.t.console':
      'Una terminal real sobre esa misma conexión SSH — <b>compartida con el agente.</b> Mientras el agente la sostiene, tu entrada queda bloqueada y se muestra un botón de Parar. <b>Nunca abre una segunda sesión que no puedas ver.</b>',
    'ui.t.band':
      '<b>Quién hizo qué.</b> Una línea por operación, etiquetada <code>[Human]</code> o <code>[AI]</code>, en el orden en que ocurrió. <b>Es la pestaña que se lee para responder «¿qué ha estado haciendo el agente?»</b>',
    'ui.t.diag':
      '<b>Por qué algo falló.</b> Las etapas de la conexión, los errores y qué hacer a continuación. <b>La pestaña que se lee cuando algo no funcionó</b> — y de la que se copia al abrir un issue.',
    'ui.confusable':
      '<b>Actividad y Registro no son lo mismo</b>, y los nombres todavía no lo dejan bastante claro. <b>Actividad es un registro de acciones</b>: responde a <i>quién hizo qué</i>. <b>Registro es un registro de diagnóstico</b>: responde a <i>por qué falló</i>. Si buscas qué ejecutó el agente, quieres Actividad. Si buscas por qué no abría una conexión, quieres Registro.',
    'h.howto': 'Cómo se usa',
    'howto.1':
      '<b>Registra un servidor.</b> Pestaña Conexiones → añadir uno. Le das un id, un nombre, un host y un usuario. <b>sshboard no guarda la frase de contraseña ni la contraseña</b> — van al almacén de credenciales del sistema, o lo dejas en manos de ssh-agent.',
    'howto.2':
      '<b>Ábrelo.</b> Se convierte en una pestaña. Pueden estar abiertos varios a la vez, y <b>todos ellos son visibles.</b> Si una clave necesita frase de contraseña, <b>la pregunta aparece en tu pantalla</b> — <b>el agente no puede responderla.</b>',
    'howto.3':
      '<b>Trabaja en Archivos o en Consola.</b> Ambas van por esa misma conexión. <b>Nada de lo que hace el agente ocurre en otra distinta.</b>',
    'howto.4':
      '<b>Apunta tu agente hacia ella</b> (más abajo). Pídele que mire algo. <b>Lo que ejecuta aparece en Actividad según ocurre</b>, y su salida aparece en <b>la misma consola que tú estás mirando.</b>',
    'howto.5':
      '<b>Lo que cambia cosas lo aprueba una persona.</b> La lectura está vallada por una <b>lista de permitidos que tú escribiste</b>. Todo lo que cambia el estado se <b>rechaza la primera vez</b> y se te <b>muestra entero</b> antes de poder ejecutarse — y la aprobación vale <b>para una ejecución, durante cinco minutos.</b>',
    'howto.empty':
      '<b>Recién instalado, el agente no puede hacer absolutamente nada.</b> La lista de permitidos de solo lectura está vacía, los directorios de escritura están vacíos y la lista de operaciones que cambian el estado está vacía. <b>No es un paso de configuración que falte: es el diseño.</b> Lo vas ensanchando justo hasta donde de verdad necesitas, línea a línea.',
    'h.tools': 'Qué puede hacer el agente',
    'tools.intro':
      '<span id="tool-count"></span> herramientas por MCP. <b>Estas son exactamente las descripciones que lee el agente</b>, por eso se muestran en inglés. Fíjate en lo que <b>no</b> está: no hay ninguna herramienta que acepte una cadena de comando arbitraria, ni ninguna que descargue a tu máquina.',
    'tools.connect': 'Conectar y elegir a dónde van las cosas',
    'tools.look': 'Mirar el servidor',
    'tools.write': 'Escribir (solo dentro de tu valla)',
    'tools.cmds': 'Comandos que tú permitiste',
    'tools.console': 'La terminal compartida',
    'tools.screen': 'La pantalla de la persona',
    'tools.itself': 'Sobre el propio sshboard',
    'h.state': 'Dónde está realmente ahora',
    'state.intro':
      '<b>Las funciones están. La interfaz no está terminada.</b> sshboard se está usando contra un servidor de producción real, y toda la lista de issues visible salió de ese uso — pero <b>todavía no ha tenido una pasada de diseño, y se nota.</b> Si buscas algo pulido, aún no lo es. Si buscas algo <b>cuyo modelo de seguridad puedas leer de principio a fin y discutir</b>, ya está listo.',
    'state.working': 'Funciona',
    'state.notyet': 'Todavía no',
    'state.working.list':
      'Una sesión SSH compartida (interfaz + agente)<br />35 herramientas MCP<br />Comandos de solo lectura desde una lista de permitidos<br />Operaciones de cambio de estado aprobadas<br /><code>sudo</code> con una contraseña que nadie almacena<br />Terminal compartida con botón de parada<br />Actualización automática',
    'state.notyet.list':
      'Una pasada de diseño sobre la interfaz<br />Capturas de pantalla en esta página<br />Firma de código<br />ssh-agent en Windows (named pipe / Pageant) — sin verificar<br />Compilaciones para macOS Intel / Linux<br />Pruebas que rendericen la pantalla misma',
    'h.fence': 'La valla',
    'fence.intro':
      '<b>Son restricciones en el código, no consejos en un prompt.</b> Se sostienen coopere o no el modelo.',
    'fence.1':
      '<b>No existe una herramienta <code>run_command(cmd)</code>.</b> Ni filtrada ni ingeniosa. <b>Un agente no puede entregarle a un shell una cadena arbitraria.</b>',
    'fence.2':
      '<b>Los comandos de solo lectura salen de una lista que tú escribiste.</b> <code>readonly.toml</code> <b>empieza vacío</b>, así que tal cual viene el agente <b>no puede ejecutar ni un solo comando.</b>',
    'fence.3':
      '<b>La escritura está vallada a los directorios que enumeraste</b>, por conexión. Esa lista <b>también empieza vacía.</b> <b>Solo subir y crear directorio</b> — nada de borrar, renombrar, mover, cambiar permisos, reiniciar servicios ni instalar paquetes.',
    'fence.4':
      '<b>No hay herramienta para descargar a tu máquina.</b> La valla <b>no protege tu portátil ni un milímetro</b>, así que el agente no recibe una puerta hacia él.',
    'fence.5':
      '<b>Las operaciones que cambian el estado necesitan que una persona pulse</b>, y el primer intento siempre se rechaza para que la pantalla pueda <b>mostrarte exactamente qué se ejecutaría.</b> Las aprobaciones <b>caducan en 5 minutos y cubren una ejecución.</b>',
    'fence.6':
      '<b>Las claves se quedan en el almacén de credenciales del sistema y en ssh-agent.</b> sshboard <b>no implementa un almacén de claves</b>, y la herramienta que lista conexiones devuelve <b>identificadores, nunca credenciales.</b>',
    'h.agent': 'Conecta tu agente',
    'agent.intro':
      'Apunta tu agente a la aplicación en ejecución. Esta forma arranca sshboard <b>como un relé sin ventana</b>, de modo que <b>el token no se escribe nunca en disco</b> — ni en <code>~/.claude.json</code> ni en el historial de tu shell.',
    'agent.note':
      'El relé no sostiene ningún motor y <b>no abre ninguna conexión propia</b> — solo lo hace la aplicación, y ese es justo el punto. <b>Es un paso de una sola vez</b>: el puerto está fijo en <code>22022</code> y el token se reutiliza, así que reiniciar no te obliga a registrarlo otra vez. Si el puerto está ocupado, <b>no se cambia en silencio a otro</b>: lo dice en pantalla. (También existe la forma HTTP simple, pero esa <b>deja el token en un archivo</b> — la ventana tiene un botón que lo copia.)',
    'agent.about':
      'Pídele primero que llame a <code>about_sshboard</code>: esa herramienta existe para que un agente averigüe por sí mismo qué es esto, qué puede y qué no puede hacer, y qué cambió en cada versión, <b>sin que tú le pegues una explicación.</b>',
    'h.never': 'En qué no se convertirá',
    'never.1': 'Una manera de hacer correr un agente contra un servidor <b>mientras nadie mira.</b>',
    'never.2': 'Un sitio donde tus claves o frases de contraseña <b>queden guardadas.</b>',
    'never.3': 'Una herramienta que abra <b>una segunda sesión SSH que no puedas ver.</b>',
    'f.source': 'Código fuente',
    'f.releases': 'Versiones',
    'f.issues': 'Issues',
  },
  fr: {
    'lede': "Vous et votre agent IA, sur une seule session SSH.",
    'tag.alpha': 'Alpha',
    'tag.mac': 'macOS (Apple Silicon)',
    'tag.win': 'Windows x64',
    'tag.langs': '11 langues',
    'intro':
      "La plupart des outils pour agents donnent au modèle un trou <code>run_command(cmd)</code> et le laissent travailler <b>là où personne ne regarde</b>. sshboard fait l'inverse : l'agent et vous tenez <b>la même connexion SSH</b>, et tout ce qu'il fait arrive sur un écran devant lequel vous êtes assis. <b>Il n'y a pas de seconde session invisible.</b>",
    'h.download': 'Télécharger',
    'dl.button': 'Télécharger',
    'dl.mac.sub': 'Apple Silicon',
    'dl.win.sub': "x64 · programme d'installation (NSIS)",
    'dl.msi.title': 'Windows (MSI)',
    'dl.msi.sub': 'Si votre organisation déploie par MSI',
    'dl.src.title': 'Code source',
    'unsigned':
      "<b>Ce n'est pas signé.</b> SmartScreen sous Windows et Gatekeeper sous macOS vous arrêteront la première fois. Sous Windows : <i>Informations complémentaires → Exécuter quand même</i>. Sous macOS : <i>clic droit sur l'app → Ouvrir</i>. <b>C'est un outil qui manipule des clés.</b> Nous savons que la première chose qu'il vous apprend est d'écarter un avertissement — <b>la signature viendra quand quelqu'un en souffrira réellement</b>, et <a href=\"https://github.com/meta-taro/sshboard/issues\">le dire dans un issue</a> est ce qui en décidera.",
    'h.install.mac': 'Installation sous macOS',
    'install.mac':
      "La version macOS est livrée en <code>.app.tar.gz</code> plutôt qu'en <code>.dmg</code>, parce que <b>c'est le même fichier que consomme la mise à jour automatique</b>. Décompressez et déplacez :",
    'install.mac.open': "Puis ouvrez-la depuis le Finder par <b>clic droit → Ouvrir</b> la première fois.",
    'h.install.win': 'Installation sous Windows',
    'install.win':
      "Lancez le <code>-setup.exe</code>. Windows a besoin de <b>WebView2</b> ; Windows 11 et les 10 récents l'ont déjà, sinon l'installateur va le chercher — <b>la première installation demande donc une connexion réseau.</b>",
    'h.ui': 'Ce que vous avez sous les yeux',
    'ui.intro':
      "Une fenêtre, cinq onglets. <b>Il n'y a pas encore de captures d'écran sur cette page.</b> Toute capture utile de ce produit contient le nom d'hôte, le nom d'utilisateur et les chemins de quelqu'un — celles qui viendront ici seront donc <b>prises sur une configuration fictive.</b> En attendant, voici sa forme.",
    'tab.connections': 'Connexions',
    'tab.files': 'Fichiers',
    'tab.console': 'Console',
    'tab.band': 'Activité',
    'tab.diag': 'Journal',
    'ui.left.title': 'À gauche',
    'ui.left': "Les serveurs que vous avez enregistrés, une ligne chacun. En ouvrir un en fait un onglet en haut de la zone de travail.",
    'ui.right.title': 'À droite',
    'ui.right': "Ce à quoi sert cet onglet — un listing de répertoire, un terminal vivant, ou l'un des deux relevés ci-dessous.",
    'ui.th.tab': 'Onglet',
    'ui.th.what': 'Ce qu’il contient',
    'ui.t.connections':
      "Les serveurs enregistrés, et le bouton qui en ouvre un. <b>Les identifiants vivent dans le trousseau du système et dans ssh-agent, jamais ici.</b>",
    'ui.t.files':
      "Deux volets : le serveur d'un côté, votre machine de l'autre. Envoyer, récupérer, créer un répertoire. <b>L'agent est clôturé aux répertoires que vous avez listés ; vous, non.</b>",
    'ui.t.console':
      "Un vrai terminal sur cette même connexion SSH — <b>partagé avec l'agent.</b> Tant que l'agent le tient, votre saisie est verrouillée et un bouton Arrêter est affiché. <b>Il n'ouvre jamais une seconde session que vous ne voyez pas.</b>",
    'ui.t.band':
      "<b>Qui a fait quoi.</b> Une ligne par opération, marquée <code>[Human]</code> ou <code>[AI]</code>, dans l'ordre où c'est arrivé. <b>C'est l'onglet qu'on lit pour répondre à « qu'a donc fait l'agent ? »</b>",
    'ui.t.diag':
      "<b>Pourquoi quelque chose a échoué.</b> Les étapes de connexion, les erreurs, et quoi faire ensuite. <b>L'onglet qu'on lit quand ça n'a pas marché</b> — et celui d'où l'on copie pour ouvrir un issue.",
    'ui.confusable':
      "<b>Activité et Journal ne sont pas la même chose</b>, et les noms ne le rendent pas encore assez évident. <b>Activité est un relevé d'actions</b> — il répond à <i>qui a fait quoi</i>. <b>Journal est un relevé de diagnostic</b> — il répond à <i>pourquoi ça a échoué</i>. Si vous cherchez ce que l'agent a exécuté : Activité. Si vous cherchez pourquoi une connexion refusait de s'ouvrir : Journal.",
    'h.howto': 'Comment on s’en sert',
    'howto.1':
      "<b>Enregistrez un serveur.</b> Onglet Connexions → en ajouter un. Vous donnez un id, un nom, un hôte et un utilisateur. <b>sshboard ne conserve jamais la phrase de passe ni le mot de passe</b> — ils vont au trousseau du système, ou vous laissez faire ssh-agent.",
    'howto.2':
      "<b>Ouvrez-le.</b> Il devient un onglet. Plusieurs peuvent être ouverts à la fois, et <b>chacun d'eux est visible.</b> Si une clé demande une phrase de passe, <b>la question apparaît sur votre écran</b> — <b>l'agent ne peut pas y répondre.</b>",
    'howto.3':
      "<b>Travaillez dans Fichiers ou dans Console.</b> Les deux passent par cette même connexion. <b>Rien de ce que fait l'agent ne se produit sur une autre.</b>",
    'howto.4':
      "<b>Branchez votre agent</b> (ci-dessous). Demandez-lui d'aller voir quelque chose. <b>Ce qu'il exécute apparaît dans Activité au fil de l'eau</b>, et sa sortie apparaît dans <b>la console que vous êtes en train de regarder.</b>",
    'howto.5':
      "<b>Ce qui change l'état demande une approbation humaine.</b> La lecture est clôturée par une <b>liste d'autorisations que vous avez écrite</b>. Tout ce qui change l'état est <b>refusé la première fois</b> et vous est <b>montré en entier</b> avant de pouvoir s'exécuter — et l'approbation vaut <b>pour une exécution, pendant cinq minutes.</b>",
    'howto.empty':
      "<b>À l'installation, l'agent ne peut strictement rien faire.</b> La liste d'autorisations en lecture est vide, les répertoires en écriture sont vides, et la liste des opérations qui changent l'état est vide. <b>Ce n'est pas une étape de configuration oubliée — c'est la conception.</b> Vous l'élargissez juste autant que nécessaire, une ligne à la fois.",
    'h.tools': 'Ce que l’agent peut faire',
    'tools.intro':
      "<span id=\"tool-count\"></span> outils via MCP. <b>Ce sont exactement les descriptions que lit l'agent</b>, elles restent donc en anglais. Notez ce qui n'y est <b>pas</b> : aucun outil ne prend une chaîne de commande arbitraire, et aucun ne télécharge vers votre machine.",
    'tools.connect': 'Se connecter et choisir la destination',
    'tools.look': 'Regarder le serveur',
    'tools.write': 'Écrire (uniquement dans votre clôture)',
    'tools.cmds': 'Les commandes que vous avez autorisées',
    'tools.console': 'Le terminal partagé',
    'tools.screen': "L'écran de l'humain",
    'tools.itself': 'À propos de sshboard lui-même',
    'h.state': 'Où ça en est vraiment',
    'state.intro':
      "<b>Les fonctions sont là. L'interface n'est pas finie.</b> sshboard est utilisé contre un vrai serveur de production, et toute la liste d'issues visible vient de cet usage — mais il <b>n'a pas encore eu de passe de conception, et ça se voit.</b> Si vous cherchez quelque chose de poli, ce n'est pas encore ça. Si vous cherchez quelque chose <b>dont le modèle de sécurité se lit de bout en bout et se discute</b>, il est prêt.",
    'state.working': 'Fonctionne',
    'state.notyet': 'Pas encore',
    'state.working.list':
      "Une session SSH partagée (interface + agent)<br />35 outils MCP<br />Commandes en lecture seule depuis une liste d'autorisations<br />Opérations de changement d'état approuvées<br /><code>sudo</code> avec un mot de passe que personne ne stocke<br />Terminal partagé avec un bouton d'arrêt<br />Mise à jour automatique",
    'state.notyet.list':
      "Une passe de conception sur l'interface<br />Des captures d'écran sur cette page<br />La signature de code<br />ssh-agent sous Windows (named pipe / Pageant) — non vérifié<br />Versions macOS Intel / Linux<br />Des tests qui rendent l'écran lui-même",
    'h.fence': 'La clôture',
    'fence.intro':
      "<b>Ce sont des contraintes dans le code, pas des conseils dans un prompt.</b> Elles tiennent, que le modèle coopère ou non.",
    'fence.1':
      "<b>Il n'y a pas d'outil <code>run_command(cmd)</code>.</b> Ni filtré, ni malin. <b>Un agent ne peut pas passer une chaîne arbitraire à un shell.</b>",
    'fence.2':
      "<b>Les commandes en lecture seule viennent d'une liste que vous avez écrite.</b> <code>readonly.toml</code> <b>démarre vide</b> : tel quel, l'agent <b>ne peut exécuter aucune commande.</b>",
    'fence.3':
      "<b>L'écriture est clôturée aux répertoires que vous avez listés</b>, par connexion. Cette liste <b>démarre vide elle aussi.</b> <b>Envoi et création de répertoire seulement</b> — ni suppression, ni renommage, ni déplacement, ni changement de droits, ni redémarrage de service, ni installation de paquet.",
    'fence.4':
      "<b>Il n'y a pas d'outil pour télécharger vers votre machine.</b> La clôture <b>ne protège pas votre portable d'un pouce</b> : l'agent n'obtient donc pas de porte vers lui.",
    'fence.5':
      "<b>Les opérations qui changent l'état demandent un appui humain</b>, et la première tentative est toujours refusée pour que l'écran puisse <b>montrer exactement ce qui s'exécuterait.</b> Les approbations <b>expirent en 5 minutes et couvrent une exécution.</b>",
    'fence.6':
      "<b>Les clés restent dans le trousseau du système et dans ssh-agent.</b> sshboard <b>n'implémente pas de magasin de clés</b>, et l'outil qui liste les connexions renvoie <b>des identifiants, jamais des secrets.</b>",
    'h.agent': 'Brancher votre agent',
    'agent.intro':
      "Pointez votre agent sur l'application en cours d'exécution. Cette forme démarre sshboard <b>en relais sans fenêtre</b>, de sorte que <b>le jeton n'est jamais écrit sur le disque</b> — ni dans <code>~/.claude.json</code>, ni dans votre historique de shell.",
    'agent.note':
      "Le relais ne détient aucun moteur et <b>n'ouvre aucune connexion propre</b> — seule l'application le fait, et c'est tout l'intérêt. <b>C'est une étape unique</b> : le port est fixé à <code>22022</code> et le jeton est réutilisé, donc redémarrer ne vous oblige pas à réenregistrer. Si le port est pris, <b>il ne bascule pas discrètement sur un autre</b> : il le dit à l'écran. (Il existe aussi une forme HTTP simple, mais celle-là <b>laisse le jeton dans un fichier</b> — la fenêtre a un bouton qui la copie.)",
    'agent.about':
      "Faites-lui d'abord appeler <code>about_sshboard</code> — cet outil existe pour qu'un agent découvre seul ce qu'est ce programme, ce qu'il a le droit de faire ou non, et ce qui a changé à chaque version, <b>sans que vous ayez à lui coller un briefing.</b>",
    'h.never': 'Ce qu’il ne deviendra pas',
    'never.1': "Un moyen de faire tourner un agent contre un serveur <b>pendant que personne ne regarde.</b>",
    'never.2': "Un endroit où vos clés ou vos phrases de passe <b>sont stockées.</b>",
    'never.3': "Un outil qui ouvre <b>une seconde session SSH que vous ne pouvez pas voir.</b>",
    'f.source': 'Code source',
    'f.releases': 'Versions',
    'f.issues': 'Issues',
  },
  de: {
    'lede': 'Sie und Ihr KI-Agent auf ein und derselben SSH-Sitzung.',
    'tag.alpha': 'Alpha',
    'tag.mac': 'macOS (Apple Silicon)',
    'tag.win': 'Windows x64',
    'tag.langs': '11 Sprachen',
    'intro':
      'Die meisten Agenten-Werkzeuge geben dem Modell ein <code>run_command(cmd)</code>-Loch und lassen es dort arbeiten, <b>wo niemand hinsieht</b>. sshboard macht das Gegenteil: Der Agent und Sie halten <b>dieselbe SSH-Verbindung</b>, und alles, was er tut, landet auf einem Bildschirm, vor dem Sie sitzen. <b>Es gibt keine zweite, unsichtbare Sitzung.</b>',
    'h.download': 'Herunterladen',
    'dl.button': 'Herunterladen',
    'dl.mac.sub': 'Apple Silicon',
    'dl.win.sub': 'x64 · Installer (NSIS)',
    'dl.msi.title': 'Windows (MSI)',
    'dl.msi.sub': 'Falls Ihre Organisation per MSI verteilt',
    'dl.src.title': 'Quellcode',
    'unsigned':
      '<b>Nicht signiert.</b> Windows SmartScreen und macOS Gatekeeper halten Sie beim ersten Mal an. Unter Windows: <i>Weitere Informationen → Trotzdem ausführen</i>. Unter macOS: <i>Rechtsklick auf die App → Öffnen</i>. <b>Dies ist ein Werkzeug, das mit Schlüsseln umgeht.</b> Uns ist bewusst, dass es Ihnen als Erstes beibringt, eine Warnung wegzuklicken — <b>signiert wird, sobald jemandem das Fehlen der Signatur tatsächlich schadet</b>, und <a href="https://github.com/meta-taro/sshboard/issues">es in einem Issue zu schreiben</a> ist es, was darüber entscheidet.',
    'h.install.mac': 'Installation unter macOS',
    'install.mac':
      'Der macOS-Build kommt als <code>.app.tar.gz</code> statt als <code>.dmg</code>, weil <b>dieselbe Datei vom automatischen Update verwendet wird</b>. Entpacken und verschieben:',
    'install.mac.open': 'Danach im Finder mit <b>Rechtsklick → Öffnen</b> das erste Mal starten.',
    'h.install.win': 'Installation unter Windows',
    'install.win':
      'Führen Sie die <code>-setup.exe</code> aus. Windows braucht <b>WebView2</b>; Windows 11 und aktuelle 10er haben es bereits, sonst holt es der Installer — die <b>erste Installation braucht also eine Netzverbindung.</b>',
    'h.ui': 'Was Sie da sehen',
    'ui.intro':
      'Ein Fenster, fünf Reiter. <b>Auf dieser Seite gibt es noch keine Screenshots.</b> Auf jedem brauchbaren Screenshot dieses Programms stehen der Hostname, der Benutzername und die Pfade von irgendjemandem — die hier werden deshalb <b>mit einer erfundenen Konfiguration aufgenommen.</b> Bis dahin hier die Form davon.',
    'tab.connections': 'Verbindungen',
    'tab.files': 'Dateien',
    'tab.console': 'Konsole',
    'tab.band': 'Aktivität',
    'tab.diag': 'Protokoll',
    'ui.left.title': 'Links',
    'ui.left': 'Die Server, die Sie eingetragen haben, je eine Zeile. Geöffnet wird daraus ein Reiter oben im Arbeitsbereich.',
    'ui.right.title': 'Rechts',
    'ui.right': 'Wofür der Reiter da ist — eine Verzeichnisliste, ein laufendes Terminal oder eine der beiden Aufzeichnungen unten.',
    'ui.th.tab': 'Reiter',
    'ui.th.what': 'Was darin steht',
    'ui.t.connections':
      'Die eingetragenen Server und der Knopf, der einen davon öffnet. <b>Zugangsdaten liegen im Anmeldeinformations-Speicher des Systems und im ssh-agent, niemals hier.</b>',
    'ui.t.files':
      'Zwei Hälften: auf der einen der Server, auf der anderen Ihr Rechner. Hochladen, herunterladen, Verzeichnis anlegen. <b>Der Agent ist auf die von Ihnen aufgezählten Verzeichnisse eingezäunt; Sie sind es nicht.</b>',
    'ui.t.console':
      'Ein echtes Terminal auf derselben SSH-Verbindung — <b>geteilt mit dem Agenten.</b> Solange der Agent es hält, ist Ihre Eingabe gesperrt und ein Stopp-Knopf sichtbar. <b>Es öffnet nie eine zweite Sitzung, die Sie nicht sehen.</b>',
    'ui.t.band':
      '<b>Wer was getan hat.</b> Eine Zeile pro Vorgang, mit <code>[Human]</code> oder <code>[AI]</code> markiert, in der Reihenfolge des Geschehens. <b>Das ist der Reiter für die Frage „Was hat der Agent eigentlich gemacht?"</b>',
    'ui.t.diag':
      '<b>Warum etwas fehlgeschlagen ist.</b> Verbindungsstufen, Fehler und was dagegen zu tun ist. <b>Der Reiter, den Sie lesen, wenn etwas nicht ging</b> — und der, aus dem Sie für ein Issue kopieren.',
    'ui.confusable':
      '<b>Aktivität und Protokoll sind nicht dasselbe</b>, und die Namen machen das noch nicht deutlich genug. <b>Aktivität ist eine Aufzeichnung von Handlungen</b> — sie beantwortet <i>wer was getan hat</i>. <b>Das Protokoll ist eine Aufzeichnung der Diagnose</b> — es beantwortet <i>warum es fehlschlug</i>. Wollen Sie wissen, was der Agent ausgeführt hat: Aktivität. Wollen Sie wissen, warum eine Verbindung nicht zustande kam: Protokoll.',
    'h.howto': 'So benutzen Sie es',
    'howto.1':
      '<b>Einen Server eintragen.</b> Reiter „Verbindungen" → einen hinzufügen. Sie geben ID, Name, Host und Benutzer an. <b>Passphrase oder Passwort speichert sshboard nicht</b> — sie gehen in den Anmeldeinformations-Speicher des Systems, oder Sie überlassen es dem ssh-agent.',
    'howto.2':
      '<b>Öffnen.</b> Daraus wird ein Reiter. Mehrere können gleichzeitig offen sein, und <b>jeder davon ist sichtbar.</b> Braucht ein Schlüssel eine Passphrase, <b>erscheint die Frage auf Ihrem Bildschirm</b> — <b>der Agent kann sie nicht beantworten.</b>',
    'howto.3':
      '<b>Arbeiten Sie in Dateien oder Konsole.</b> Beides läuft über dieselbe Verbindung. <b>Nichts, was der Agent tut, geschieht auf einer anderen.</b>',
    'howto.4':
      '<b>Richten Sie Ihren Agenten darauf</b> (siehe unten). Lassen Sie ihn etwas nachsehen. <b>Was er ausführt, erscheint laufend in der Aktivität</b>, und seine Ausgabe erscheint in <b>derselben Konsole, die Sie ansehen.</b>',
    'howto.5':
      '<b>Was etwas verändert, muss ein Mensch freigeben.</b> Lesen ist durch eine <b>von Ihnen geschriebene Positivliste</b> eingezäunt. Alles, was den Zustand ändert, wird <b>beim ersten Mal abgelehnt</b> und Ihnen <b>vollständig gezeigt</b>, bevor es laufen darf — und die Freigabe gilt <b>für einen Lauf, fünf Minuten lang.</b>',
    'howto.empty':
      '<b>Direkt nach der Installation kann der Agent überhaupt nichts.</b> Die Positivliste fürs Lesen ist leer, die Schreibverzeichnisse sind leer, und die Liste zustandsändernder Vorgänge ist leer. <b>Das ist kein vergessener Einrichtungsschritt — das ist der Entwurf.</b> Sie weiten es so weit aus, wie Sie es wirklich brauchen, Zeile für Zeile.',
    'h.tools': 'Was der Agent tun kann',
    'tools.intro':
      '<span id="tool-count"></span> Werkzeuge über MCP. <b>Das hier sind genau die Beschreibungen, die der Agent liest</b>, deshalb stehen sie auf Englisch. Beachten Sie, was <b>nicht</b> dabei ist: kein Werkzeug, das eine beliebige Befehlszeichenkette entgegennimmt, und keines, das auf Ihren Rechner herunterlädt.',
    'tools.connect': 'Verbinden und festlegen, wohin es geht',
    'tools.look': 'Auf den Server schauen',
    'tools.write': 'Schreiben (nur innerhalb Ihres Zauns)',
    'tools.cmds': 'Befehle, die Sie erlaubt haben',
    'tools.console': 'Das geteilte Terminal',
    'tools.screen': 'Der Bildschirm des Menschen',
    'tools.itself': 'Über sshboard selbst',
    'h.state': 'Wo es gerade wirklich steht',
    'state.intro':
      '<b>Die Funktionen sind da. Die Oberfläche ist nicht fertig.</b> sshboard wird gegen einen echten Produktionsserver eingesetzt, und die gesamte sichtbare Issue-Liste stammt aus diesem Einsatz — aber es hat <b>noch keinen Gestaltungsdurchgang gehabt, und das sieht man.</b> Wer etwas Poliertes sucht: noch nicht. Wer etwas sucht, <b>dessen Sicherheitsmodell sich von Anfang bis Ende lesen und bestreiten lässt</b>: das können Sie jetzt.',
    'state.working': 'Funktioniert',
    'state.notyet': 'Noch nicht',
    'state.working.list':
      'Eine geteilte SSH-Sitzung (Oberfläche + Agent)<br />35 MCP-Werkzeuge<br />Nur-Lesen-Befehle aus einer Positivliste<br />Freigegebene zustandsändernde Vorgänge<br /><code>sudo</code> mit einem Passwort, das niemand speichert<br />Geteiltes Terminal mit Stopp-Knopf<br />Automatisches Update',
    'state.notyet.list':
      'Ein Gestaltungsdurchgang für die Oberfläche<br />Screenshots auf dieser Seite<br />Code-Signierung<br />ssh-agent unter Windows (Named Pipe / Pageant) — ungeprüft<br />Intel-macOS- / Linux-Builds<br />Tests, die den Bildschirm selbst rendern',
    'h.fence': 'Der Zaun',
    'fence.intro':
      '<b>Das sind Beschränkungen im Code, keine Bitten in einem Prompt.</b> Sie gelten, ob das Modell mitspielt oder nicht.',
    'fence.1':
      '<b>Es gibt kein <code>run_command(cmd)</code>-Werkzeug.</b> Kein gefiltertes, kein cleveres. <b>Ein Agent kann einer Shell keine beliebige Zeichenkette übergeben.</b>',
    'fence.2':
      '<b>Nur-Lesen-Befehle stammen aus einer Liste, die Sie geschrieben haben.</b> <code>readonly.toml</code> ist <b>anfangs leer</b> — ab Werk kann der Agent also <b>keinen einzigen Befehl ausführen.</b>',
    'fence.3':
      '<b>Schreiben ist auf die von Ihnen aufgezählten Verzeichnisse eingezäunt</b>, pro Verbindung. Auch diese Liste ist <b>anfangs leer.</b> <b>Nur Hochladen und Verzeichnis anlegen</b> — kein Löschen, Umbenennen, Verschieben, Rechte ändern, Dienst neu starten oder Paket installieren.',
    'fence.4':
      '<b>Es gibt kein Werkzeug zum Herunterladen auf Ihren Rechner.</b> Der Zaun <b>schützt Ihr Notebook kein Stück</b> — also bekommt der Agent keine Tür dorthin.',
    'fence.5':
      '<b>Zustandsändernde Vorgänge brauchen einen menschlichen Tastendruck</b>, und der erste Versuch wird immer abgelehnt, damit der Bildschirm <b>genau zeigen kann, was laufen würde.</b> Freigaben <b>verfallen nach 5 Minuten und decken einen Lauf.</b>',
    'fence.6':
      '<b>Schlüssel bleiben im Anmeldeinformations-Speicher des Systems und im ssh-agent.</b> sshboard <b>baut keinen eigenen Schlüsselspeicher</b>, und das Werkzeug, das Verbindungen auflistet, liefert <b>Bezeichner zurück, niemals Zugangsdaten.</b>',
    'h.agent': 'Ihren Agenten anbinden',
    'agent.intro':
      'Richten Sie Ihren Agenten auf die laufende App. Diese Form startet sshboard als <b>Weiterleitung ohne Fenster</b>, sodass <b>das Token nie auf die Platte geschrieben wird</b> — weder in <code>~/.claude.json</code> noch in Ihre Shell-Historie.',
    'agent.note':
      'Die Weiterleitung hält keine Engine und <b>öffnet keine eigene Verbindung</b> — das tut nur die App, und genau darum geht es. <b>Es ist ein einmaliger Schritt</b>: Der Port liegt fest auf <code>22022</code> und das Token wird wiederverwendet, ein Neustart zwingt Sie also nicht zu erneuter Registrierung. Ist der Port belegt, <b>weicht es nicht still auf einen anderen aus</b>, sondern sagt es auf dem Bildschirm. (Es gibt auch die einfache HTTP-Form, aber die <b>lässt das Token in einer Datei</b> — im Fenster gibt es einen Knopf, der sie kopiert.)',
    'agent.about':
      'Lassen Sie ihn zuerst <code>about_sshboard</code> aufrufen — dieses Werkzeug gibt es, damit ein Agent selbst herausfindet, was das hier ist, was er darf und was nicht und was sich in welcher Version geändert hat, <b>ohne dass Sie ihm eine Einweisung hineinkopieren.</b>',
    'h.never': 'Was es nicht werden wird',
    'never.1': 'Eine Möglichkeit, einen Agenten gegen einen Server laufen zu lassen, <b>während niemand zusieht.</b>',
    'never.2': 'Ein Ort, an dem Ihre Schlüssel oder Passphrasen <b>gespeichert werden.</b>',
    'never.3': 'Ein Werkzeug, das eine <b>zweite SSH-Sitzung öffnet, die Sie nicht sehen können.</b>',
    'f.source': 'Quellcode',
    'f.releases': 'Releases',
    'f.issues': 'Issues',
  },
  'zh-TW': {
    'lede': '人與 AI 代理程式共用同一條 SSH 連線。',
    'tag.alpha': '測試版',
    'tag.mac': 'macOS（Apple 晶片）',
    'tag.win': 'Windows x64',
    'tag.langs': '11 種語言',
    'intro':
      '多數工具給模型開一個 <code>run_command(cmd)</code> 的口子，讓它在<b>沒人看著的地方</b>做事。sshboard 反過來：<b>代理程式和你握住同一條 SSH 連線</b>，它做的每一件事都出現在<b>你正對著的畫面上</b>。<b>不會有第二條看不見的工作階段。</b>',
    'h.download': '下載',
    'dl.button': '下載',
    'dl.mac.sub': 'Apple 晶片',
    'dl.win.sub': 'x64 · 安裝程式（NSIS）',
    'dl.msi.title': 'Windows（MSI）',
    'dl.msi.sub': '若貴單位以 MSI 佈署',
    'dl.src.title': '原始碼',
    'unsigned':
      '<b>沒有程式碼簽章。</b>第一次開啟時，Windows SmartScreen 與 macOS Gatekeeper 都會擋下來。Windows：<i>其他資訊 → 仍要執行</i>；macOS：<i>在 App 上按右鍵 → 開啟</i>。<b>這是一個管金鑰的工具。</b>我們清楚它教你的第一件事就是「忽略警告」，仍然這樣發出來 —— <b>簽章會在真的有人因為沒簽章而受困之後</b>再加上。<a href="https://github.com/meta-taro/sshboard/issues">把遇到的狀況寫進 issue</a> 就是做這個判斷的依據。',
    'h.install.mac': '在 macOS 上安裝',
    'install.mac':
      'macOS 版打包成 <code>.app.tar.gz</code> 而不是 <code>.dmg</code>，因為<b>自動更新用的就是同一個檔案</b>。解壓後搬過去：',
    'install.mac.open': '然後在 Finder 裡<b>按右鍵 → 開啟</b>，第一次這樣開。',
    'h.install.win': '在 Windows 上安裝',
    'install.win':
      '執行 <code>-setup.exe</code>。需要 <b>WebView2</b>；Windows 11 與較新的 10 已內建，沒有的話安裝程式會去取 —— 所以<b>第一次安裝需要連網。</b>',
    'h.ui': '你看到的是什麼',
    'ui.intro':
      '一個視窗，五個分頁。<b>這個頁面上還沒有螢幕截圖。</b>因為這個產品的畫面裡一定會拍到某個人的主機名稱、使用者名稱與路徑，所以放上來的截圖會<b>在虛構的設定上拍。</b>在那之前，先把形狀放在這裡。',
    'tab.connections': '連線',
    'tab.files': '檔案',
    'tab.console': '終端機',
    'tab.band': '操作紀錄',
    'tab.diag': '日誌',
    'ui.left.title': '左側',
    'ui.left': '你註冊的伺服器，一列一個。打開後會成為工作區上方的一個分頁。',
    'ui.right.title': '右側',
    'ui.right': '那個分頁的內容 —— 目錄清單、正在執行的終端機，或下面兩種紀錄之一。',
    'ui.th.tab': '分頁',
    'ui.th.what': '裡面有什麼',
    'ui.t.connections':
      '你註冊的伺服器，以及開啟它的按鈕。<b>憑證在作業系統的憑證存放區與 ssh-agent 裡，從不放在這裡。</b>',
    'ui.t.files':
      '兩欄：一邊是伺服器，一邊是你的電腦。上傳、下載、建立目錄。<b>代理程式被限制在你列出的目錄之下</b>，<b>你自己不受限制。</b>',
    'ui.t.console':
      '同一條 SSH 連線上的真實終端機 —— <b>與代理程式共用。</b>代理程式握著它的時候你的輸入會被鎖住，並會顯示一個「停止」按鈕。<b>它絕不會另開一條你看不見的工作階段。</b>',
    'ui.t.band':
      '<b>誰做了什麼。</b>每個操作一列，帶 <code>[Human]</code> 或 <code>[AI]</code> 標記，依發生順序排列。<b>想知道「代理程式一直在做什麼」，就讀這一頁。</b>',
    'ui.t.diag':
      '<b>為什麼失敗了。</b>連線經過的階段、錯誤，以及下一步該怎麼做。<b>出問題時讀這一頁</b>，<b>要貼進 issue 的內容也從這裡複製。</b>',
    'ui.confusable':
      '<b>「操作紀錄」和「日誌」不是同一回事</b>，現在的名稱還沒把這點說清楚。<b>操作紀錄是行為的紀錄</b> —— 回答<i>誰做了什麼</i>。<b>日誌是診斷的紀錄</b> —— 回答<i>為什麼失敗</i>。<b>想知道代理程式執行了什麼，看操作紀錄；想知道連線為什麼開不起來，看日誌。</b>',
    'h.howto': '怎麼用',
    'howto.1':
      '<b>註冊一台伺服器。</b>在「連線」分頁新增一個：id、名稱、主機、使用者。<b>sshboard 不會保存密語或密碼</b> —— 它們會進作業系統的憑證存放區，或交給 ssh-agent。',
    'howto.2':
      '<b>開啟它。</b>它會成為一個分頁。可以同時開好幾個，<b>每一個都在畫面上看得見。</b>如果金鑰需要密語，<b>問題會出現在你的畫面上</b> —— <b>代理程式無法作答。</b>',
    'howto.3':
      '<b>在「檔案」或「終端機」裡工作。</b>兩者都走同一條連線。<b>不會出現代理程式在另一條連線上做事的情況。</b>',
    'howto.4':
      '<b>把你的代理程式接上來</b>（見下文）。請它去看點什麼。<b>它執行的東西會即時出現在「操作紀錄」裡</b>，輸出則出現在<b>你正在看的那個終端機</b>裡。',
    'howto.5':
      '<b>會改變狀態的事情要由人核准。</b>讀取被<b>你自己寫的允許清單</b>圈住。任何改變狀態的操作<b>第一次一定被拒絕</b>，並把<b>將要執行的內容原樣顯示給你</b>之後才會動。<b>一次核准只管一次執行，五分鐘過期。</b>',
    'howto.empty':
      '<b>剛裝好時，代理程式什麼都做不了。</b>唯讀允許清單是空的，可寫目錄是空的，會改變狀態的操作清單也是空的。<b>這不是漏設了一步，而是就這麼決定的。</b>照你真正需要的範圍，一行一行放寬。',
    'h.tools': '代理程式能做什麼',
    'tools.intro':
      '透過 MCP 提供 <span id="tool-count"></span> 個工具。<b>這裡顯示的就是代理程式實際讀到的說明原文</b>，所以保持英文。請注意<b>沒有</b>什麼 —— <b>沒有接收任意命令字串的工具，也沒有下載到你本機的工具。</b>',
    'tools.connect': '連線，以及選擇操作送往何處',
    'tools.look': '查看伺服器',
    'tools.write': '寫入（只在圍籬之內）',
    'tools.cmds': '你允許的命令',
    'tools.console': '共用的終端機',
    'tools.screen': '人的畫面',
    'tools.itself': '關於 sshboard 本身',
    'h.state': '目前到了哪一步',
    'state.intro':
      '<b>功能齊了，介面還沒打磨。</b>sshboard 已經在一台真實的正式伺服器上使用，<b>看得到的 issue 全部來自那裡</b>。但<b>還沒有做過一次設計打磨，一看就看得出來。</b>如果你要的是打磨好的東西，現在還不是。如果你要的是<b>能從頭到尾讀懂並質疑其安全模型</b>的東西，它已經可以讀了。',
    'state.working': '能用的',
    'state.notyet': '還沒有的',
    'state.working.list':
      '共用的一條 SSH 工作階段（介面 + 代理程式）<br />35 個 MCP 工具<br />來自允許清單的唯讀命令<br />經人核准的狀態變更操作<br />沒有任何地方保存的 <code>sudo</code> 密碼<br />附停止按鈕的共用終端機<br />自動更新',
    'state.notyet.list':
      '介面的設計打磨<br />這個頁面上的螢幕截圖<br />程式碼簽章<br />Windows 上的 ssh-agent（具名管道 / Pageant）—— 未驗證<br />Intel macOS / Linux 版本<br />真正繪製介面來驗證的測試',
    'h.fence': '圍籬',
    'fence.intro':
      '<b>這些是程式碼層面的約束，不是提示詞裡的請求。</b>無論模型配不配合，它們都成立。',
    'fence.1':
      '<b>沒有 <code>run_command(cmd)</code> 這類工具。</b>沒有過濾版的，也沒有聰明版的。<b>代理程式根本沒有把任意字串交給 shell 的通道。</b>',
    'fence.2':
      '<b>唯讀命令來自你自己寫的清單。</b><code>readonly.toml</code> <b>預設是空的</b>，所以剛裝好時代理程式<b>一條命令都跑不了。</b>',
    'fence.3':
      '<b>寫入被限制在你為每條連線列出的目錄之下。</b>這份清單<b>預設也是空的。</b><b>只能上傳和建立目錄</b> —— 刪除、重新命名、移動、改權限、重啟服務、安裝套件都不在其中。',
    'fence.4':
      '<b>沒有「下載到我電腦上」的工具。</b>圍籬<b>完全保護不了你的筆電</b>，所以不給代理程式開這扇門。',
    'fence.5':
      '<b>會改變狀態的操作需要人去按。</b>第一次嘗試總是被拒絕，好讓畫面把<b>將要執行的內容原樣顯示出來</b>。核准<b>五分鐘過期，只管一次執行。</b>',
    'fence.6':
      '<b>金鑰留在作業系統的憑證存放區與 ssh-agent 裡。</b>sshboard <b>不實作自己的金鑰存放</b>。列出連線的那個工具回傳的<b>只有識別碼，絕不回傳憑證。</b>',
    'h.agent': '接上你的代理程式',
    'agent.intro':
      '把代理程式指向正在執行的應用程式。這種寫法會把 sshboard <b>當成沒有視窗的中繼啟動</b>，因此 <b>權杖一次也不會寫到磁碟上</b> —— 不進 <code>~/.claude.json</code>，也不進你的 shell 紀錄。',
    'agent.note':
      '中繼本身不持有引擎，也<b>不會自己開啟任何連線</b> —— 開啟連線的只有應用程式，這正是重點。<b>這是一次性的步驟</b>：連接埠固定在 <code>22022</code>，權杖也重複使用，所以<b>重新啟動後不必重新註冊。</b>連接埠被佔用時，它<b>不會悄悄換一個號碼</b>，而是在畫面上說明。（也有單純的 HTTP 方式，但那種會<b>把權杖留在檔案裡</b> —— 視窗裡有個按鈕可以複製它。）',
    'agent.about':
      '先讓它呼叫 <code>about_sshboard</code>。它會回答<b>這是什麼、你作為 AI 可以與不可以做什麼、每個版本改了什麼</b> —— 這個工具存在的目的，就是<b>讓你不必貼上一段說明。</b>',
    'h.never': '它不會變成什麼',
    'never.1': '一種<b>在沒人看著的時候</b>把代理程式放到伺服器上跑的辦法。',
    'never.2': '一個<b>存放</b>你的金鑰或密語的地方。',
    'never.3': '一個會開啟<b>你看不見的第二條</b> SSH 工作階段的工具。',
    'f.source': '原始碼',
    'f.releases': '發行版本',
    'f.issues': 'Issues',
  },
  'zh-CN': {
    'lede': '人与 AI 代理共用同一条 SSH 连接。',
    'tag.alpha': '内测版',
    'tag.mac': 'macOS（Apple 芯片）',
    'tag.win': 'Windows x64',
    'tag.langs': '11 种语言',
    'intro':
      '大多数工具给模型开一个 <code>run_command(cmd)</code> 的口子，让它在<b>没人看着的地方</b>干活。sshboard 反过来：<b>代理和你握住同一条 SSH 连接</b>，它做的每一件事都出现在<b>你正对着的屏幕上</b>。<b>不会有第二条看不见的会话。</b>',
    'h.download': '下载',
    'dl.button': '下载',
    'dl.mac.sub': 'Apple 芯片',
    'dl.win.sub': 'x64 · 安装程序（NSIS）',
    'dl.msi.title': 'Windows（MSI）',
    'dl.msi.sub': '如果贵组织用 MSI 部署',
    'dl.src.title': '源码',
    'unsigned':
      '<b>没有代码签名。</b>第一次打开时，Windows SmartScreen 和 macOS Gatekeeper 都会拦住你。Windows：<i>更多信息 → 仍要运行</i>；macOS：<i>右键点应用 → 打开</i>。<b>这是一个管钥匙的工具。</b>我们清楚它教给你的第一件事就是"忽略警告"，仍然这样发出来 —— <b>签名会在真的有人因为没签名而受困之后</b>加上。<a href="https://github.com/meta-taro/sshboard/issues">把遇到的情况写进 issue</a> 就是做这个判断的依据。',
    'h.install.mac': '在 macOS 上安装',
    'install.mac':
      'macOS 版打包成 <code>.app.tar.gz</code> 而不是 <code>.dmg</code>，因为<b>自动更新用的就是同一个文件</b>。解压后移过去：',
    'install.mac.open': '然后在访达里<b>右键 → 打开</b>，第一次这样开。',
    'h.install.win': '在 Windows 上安装',
    'install.win':
      '运行 <code>-setup.exe</code>。需要 <b>WebView2</b>；Windows 11 和较新的 10 已经自带，没有的话安装程序会去取 —— 所以<b>第一次安装需要联网。</b>',
    'h.ui': '你看到的是什么',
    'ui.intro':
      '一个窗口，五个标签页。<b>这个页面上还没有截图。</b>因为这个产品的画面里一定会拍到某个人的主机名、用户名和路径，所以放上来的截图会<b>在虚构的配置上拍。</b>在那之前，先把形状放在这里。',
    'tab.connections': '连接',
    'tab.files': '文件',
    'tab.console': '终端',
    'tab.band': '操作记录',
    'tab.diag': '日志',
    'ui.left.title': '左侧',
    'ui.left': '你注册的服务器，一行一个。打开后会变成工作区顶部的一个标签页。',
    'ui.right.title': '右侧',
    'ui.right': '那个标签页的内容 —— 目录列表、正在运行的终端，或者下面两种记录之一。',
    'ui.th.tab': '标签页',
    'ui.th.what': '里面是什么',
    'ui.t.connections':
      '你注册的服务器，以及打开它的按钮。<b>凭据在操作系统的凭据库和 ssh-agent 里，从不放在这里。</b>',
    'ui.t.files':
      '两栏：一边是服务器，一边是你的机器。上传、下载、新建目录。<b>代理被限制在你列出的目录之下</b>，<b>你自己不受限制。</b>',
    'ui.t.console':
      '同一条 SSH 连接上的真实终端 —— <b>和代理共用。</b>代理握着它的时候你的输入会被锁住，并且会显示一个"停止"按钮。<b>它绝不会另开一条你看不见的会话。</b>',
    'ui.t.band':
      '<b>谁做了什么。</b>每个操作一行，带 <code>[Human]</code> 或 <code>[AI]</code> 标记，按发生顺序排列。<b>想知道"代理一直在干什么"，就读这一页。</b>',
    'ui.t.diag':
      '<b>为什么失败了。</b>连接经过的阶段、错误，以及下一步该怎么办。<b>出问题时读这一页</b>，<b>往 issue 里贴的内容也从这里复制。</b>',
    'ui.confusable':
      '<b>"操作记录"和"日志"不是一回事</b>，现在的名字还没把这点说清楚。<b>操作记录是行为的记录</b> —— 回答<i>谁做了什么</i>。<b>日志是诊断的记录</b> —— 回答<i>为什么失败</i>。<b>想知道代理执行了什么，看操作记录；想知道连接为什么打不开，看日志。</b>',
    'h.howto': '怎么用',
    'howto.1':
      '<b>注册一台服务器。</b>在"连接"页里添加一个：id、名称、主机、用户。<b>sshboard 不保存口令或密码</b> —— 它们进操作系统的凭据库，或者交给 ssh-agent。',
    'howto.2':
      '<b>打开它。</b>它会变成一个标签页。可以同时开好几个，<b>每一个都在屏幕上看得见。</b>如果密钥需要口令，<b>问题会出现在你的屏幕上</b> —— <b>代理无法作答。</b>',
    'howto.3':
      '<b>在"文件"或"终端"里干活。</b>两者都走同一条连接。<b>不会出现代理在另一条连接上做事的情况。</b>',
    'howto.4':
      '<b>把你的代理接上来</b>（见下文）。让它去看点什么。<b>它执行的东西会实时出现在"操作记录"里</b>，输出出现在<b>你正在看的那个终端</b>里。',
    'howto.5':
      '<b>改变状态的事情要人来批准。</b>读取被<b>你自己写的白名单</b>圈住。任何改变状态的操作<b>第一次一定被拒绝</b>，并把<b>将要执行的内容原样显示给你</b>之后才会动。<b>一次批准只管一次执行，五分钟过期。</b>',
    'howto.empty':
      '<b>刚装好时，代理什么都做不了。</b>只读白名单是空的，可写目录是空的，改变状态的操作清单也是空的。<b>这不是漏配了一步，而是就这么设计的。</b>按你真正需要的范围，一行一行地放宽。',
    'h.tools': '代理能做什么',
    'tools.intro':
      '通过 MCP 提供 <span id="tool-count"></span> 个工具。<b>这里显示的就是代理实际读到的说明原文</b>，所以保持英文。请注意<b>没有</b>什么 —— <b>没有接收任意命令字符串的工具，也没有下载到你本机的工具。</b>',
    'tools.connect': '连接，以及选择操作发往哪里',
    'tools.look': '查看服务器',
    'tools.write': '写入（只在围栏之内）',
    'tools.cmds': '你允许的命令',
    'tools.console': '共用的终端',
    'tools.screen': '人的屏幕',
    'tools.itself': '关于 sshboard 本身',
    'h.state': '目前到了哪一步',
    'state.intro':
      '<b>功能齐了，界面还没打磨。</b>sshboard 已经在一台真实的生产服务器上使用，<b>能看到的 issue 全部来自那里</b>。但<b>还没有做过一次设计打磨，一看就能看出来。</b>如果你要的是打磨好的东西，现在还不是。如果你要的是<b>能从头到尾读懂并质疑其安全模型</b>的东西，它已经可以读了。',
    'state.working': '能用的',
    'state.notyet': '还没有的',
    'state.working.list':
      '共用的一条 SSH 会话（界面 + 代理）<br />35 个 MCP 工具<br />来自白名单的只读命令<br />经人批准的状态变更操作<br />没有任何地方保存的 <code>sudo</code> 密码<br />带停止按钮的共用终端<br />自动更新',
    'state.notyet.list':
      '界面的设计打磨<br />这个页面上的截图<br />代码签名<br />Windows 上的 ssh-agent（命名管道 / Pageant）—— 未验证<br />Intel macOS / Linux 版本<br />真正渲染界面来验证的测试',
    'h.fence': '围栏',
    'fence.intro':
      '<b>这些是代码层面的约束，不是提示词里的请求。</b>无论模型配不配合，它们都成立。',
    'fence.1':
      '<b>没有 <code>run_command(cmd)</code> 这类工具。</b>没有过滤版的，也没有聪明版的。<b>代理根本没有把任意字符串交给 shell 的通道。</b>',
    'fence.2':
      '<b>只读命令来自你自己写的清单。</b><code>readonly.toml</code> <b>默认是空的</b>，所以刚装好时代理<b>一条命令都跑不了。</b>',
    'fence.3':
      '<b>写入被限制在你为每条连接列出的目录之下。</b>这个清单<b>默认也是空的。</b><b>只能上传和新建目录</b> —— 删除、重命名、移动、改权限、重启服务、装包都不在其中。',
    'fence.4':
      '<b>没有"下载到我机器上"的工具。</b>围栏<b>完全保护不了你的笔记本</b>，所以不给代理开这扇门。',
    'fence.5':
      '<b>改变状态的操作需要人去按。</b>第一次尝试总是被拒绝，好让屏幕把<b>将要执行的内容原样显示出来</b>。批准<b>五分钟过期，只管一次执行。</b>',
    'fence.6':
      '<b>密钥留在操作系统的凭据库和 ssh-agent 里。</b>sshboard <b>不实现自己的密钥存储</b>。列出连接的那个工具返回的<b>只有标识符，绝不返回凭据。</b>',
    'h.agent': '接上你的代理',
    'agent.intro':
      '把代理指向正在运行的应用。这种写法会把 sshboard <b>作为没有窗口的中继启动</b>，因此 <b>令牌一次也不会写到磁盘上</b> —— 不进 <code>~/.claude.json</code>，也不进你的 shell 历史。',
    'agent.note':
      '中继本身不持有引擎，也<b>不会自己打开任何连接</b> —— 打开连接的只有应用，这正是要点。<b>这是一次性的步骤</b>：端口固定在 <code>22022</code>，令牌也复用，所以<b>重启之后不用重新注册。</b>端口被占用时，它<b>不会悄悄换一个号</b>，而是在屏幕上说明。（也有普通的 HTTP 方式，但那种会<b>把令牌留在文件里</b> —— 窗口里有个按钮可以复制它。）',
    'agent.about':
      '先让它调用 <code>about_sshboard</code>。它会回答<b>这是什么、你作为 AI 可以和不可以做什么、每个版本改了什么</b> —— 这个工具存在的目的，就是<b>让你不必粘贴一段说明。</b>',
    'h.never': '它不会变成什么',
    'never.1': '一种<b>在没人看着的时候</b>把代理放到服务器上跑的办法。',
    'never.2': '一个<b>存放</b>你的密钥或口令的地方。',
    'never.3': '一个会打开<b>你看不见的第二条</b> SSH 会话的工具。',
    'f.source': '源码',
    'f.releases': '发布',
    'f.issues': 'Issues',
  },
  ko: {
    'lede': '사람과 AI 에이전트가 하나의 SSH 세션을 함께 씁니다.',
    'tag.alpha': '알파',
    'tag.mac': 'macOS (Apple Silicon)',
    'tag.win': 'Windows x64',
    'tag.langs': '11개 언어',
    'intro':
      '대부분의 도구는 모델에게 <code>run_command(cmd)</code> 구멍을 주고 <b>아무도 보지 않는 곳</b>에서 일하게 합니다. sshboard는 반대입니다. <b>에이전트와 사람이 같은 SSH 연결 하나</b>를 쥐고, 에이전트가 하는 모든 일이 <b>사람이 보고 있는 화면</b>에 나타납니다. <b>보이지 않는 두 번째 세션은 열지 않습니다.</b>',
    'h.download': '내려받기',
    'dl.button': '내려받기',
    'dl.mac.sub': 'Apple Silicon',
    'dl.win.sub': 'x64 · 설치 프로그램 (NSIS)',
    'dl.msi.title': 'Windows (MSI)',
    'dl.msi.sub': '조직에서 MSI로 배포하는 경우',
    'dl.src.title': '소스',
    'unsigned':
      '<b>코드 서명이 되어 있지 않습니다.</b> 처음 열 때 Windows SmartScreen과 macOS Gatekeeper가 막습니다. Windows는 <i>추가 정보 → 실행</i>, macOS는 <i>앱 우클릭 → 열기</i>입니다. <b>이것은 키를 다루는 도구입니다.</b> 가장 먼저 가르치는 것이 "경고를 무시하고 열기"가 된다는 점을 알면서도 내놓습니다 —— <b>서명은 서명이 없어서 실제로 곤란한 사람이 나온 뒤에</b> 넣습니다. <a href="https://github.com/meta-taro/sshboard/issues">이슈에 적어 주는 것</a>이 그 판단 근거가 됩니다.',
    'h.install.mac': 'macOS에 설치',
    'install.mac':
      'macOS 빌드는 <code>.dmg</code>가 아니라 <code>.app.tar.gz</code>입니다. <b>같은 파일을 자동 업데이트가 사용</b>하기 때문입니다. 풀어서 옮기세요.',
    'install.mac.open': '그다음 Finder에서 <b>우클릭 → 열기</b>로 처음 한 번 엽니다.',
    'h.install.win': 'Windows에 설치',
    'install.win':
      '<code>-setup.exe</code>를 실행합니다. <b>WebView2</b>가 필요합니다. Windows 11과 최근 10에는 들어 있고, 없으면 설치 프로그램이 받아오므로 <b>첫 설치에는 인터넷 연결이 필요합니다.</b>',
    'h.ui': '화면에 보이는 것',
    'ui.intro':
      '창 하나, 탭 다섯 개입니다. <b>이 페이지에는 아직 화면 사진이 없습니다.</b> 이 제품의 화면에는 반드시 누군가의 호스트명·사용자명·경로가 찍히기 때문이며, 여기에 올릴 것은 <b>가상의 설정 위에서 찍습니다.</b> 그때까지는 형태만 둡니다.',
    'tab.connections': '연결',
    'tab.files': '파일',
    'tab.console': '터미널',
    'tab.band': '작업 기록',
    'tab.diag': '로그',
    'ui.left.title': '왼쪽',
    'ui.left': '등록한 서버가 한 줄씩. 열면 작업 영역 위쪽에 탭으로 늘어섭니다.',
    'ui.right.title': '오른쪽',
    'ui.right': '그 탭의 내용 —— 디렉터리 목록, 살아 있는 터미널, 또는 아래 두 가지 기록.',
    'ui.th.tab': '탭',
    'ui.th.what': '무엇이 있는가',
    'ui.t.connections':
      '등록한 서버와 여는 버튼. <b>자격 정보는 OS 자격 증명 저장소와 ssh-agent에 있고, 여기에는 두지 않습니다.</b>',
    'ui.t.files':
      '두 개의 면. 한쪽은 서버, 다른 쪽은 내 컴퓨터. 올리기·내려받기·디렉터리 만들기. <b>에이전트는 사람이 나열한 디렉터리 아래에만</b> 쓸 수 있고, <b>사람은 제한되지 않습니다.</b>',
    'ui.t.console':
      '같은 SSH 연결 위의 진짜 터미널 —— <b>에이전트와 함께 씁니다.</b> 에이전트가 쥐고 있는 동안에는 사람의 입력이 잠기고 [멈춤] 버튼이 보입니다. <b>보이지 않는 두 번째 세션은 열지 않습니다.</b>',
    'ui.t.band':
      '<b>누가 무엇을 했는가.</b> 작업 하나에 한 줄, <code>[Human]</code> 또는 <code>[AI]</code> 표시와 함께 일어난 순서대로. <b>"에이전트가 무엇을 하고 있었나"를 읽는 탭</b>입니다.',
    'ui.t.diag':
      '<b>왜 실패했는가.</b> 연결 단계, 오류, 그리고 다음에 할 일. <b>잘 안 됐을 때 읽는 탭</b>이고, <b>이슈에 붙여 넣을 곳</b>도 여기입니다.',
    'ui.confusable':
      '<b>"작업 기록"과 "로그"는 서로 다른 것입니다.</b> 지금 이름은 그 점을 충분히 전하지 못합니다. <b>작업 기록은 행동의 기록</b> —— <i>누가 무엇을 했는가</i>에 답합니다. <b>로그는 진단의 기록</b> —— <i>왜 실패했는가</i>에 답합니다. <b>에이전트가 무엇을 실행했는지 알고 싶으면 작업 기록, 연결이 왜 안 되는지 알고 싶으면 로그</b>입니다.',
    'h.howto': '사용법',
    'howto.1':
      '<b>서버를 등록합니다.</b> [연결] 탭에서 하나 추가합니다. id, 이름, 호스트, 사용자를 적습니다. <b>패스프레이즈나 비밀번호를 sshboard는 저장하지 않습니다</b> —— OS 자격 증명 저장소로 가거나, ssh-agent에 맡깁니다.',
    'howto.2':
      '<b>엽니다.</b> 탭이 됩니다. 여러 개를 동시에 열 수 있고 <b>그 전부가 화면에 보입니다.</b> 키에 패스프레이즈가 필요하면 <b>질문이 사람의 화면에 뜹니다</b> —— <b>에이전트는 답할 수 없습니다.</b>',
    'howto.3':
      '<b>[파일]이나 [터미널]에서 작업합니다.</b> 둘 다 같은 연결 위입니다. <b>에이전트가 하는 일이 다른 연결에서 일어나는 일은 없습니다.</b>',
    'howto.4':
      '<b>에이전트를 연결합니다</b>(아래). 무언가 살펴보라고 시켜 보세요. <b>실행한 것은 [작업 기록]에 즉시 나오고</b>, 출력은 <b>사람이 보고 있는 바로 그 터미널</b>에 나옵니다.',
    'howto.5':
      '<b>상태를 바꾸는 것은 사람이 승인합니다.</b> 읽기는 <b>사람이 쓴 허용 목록</b>으로 둘러싸여 있습니다. 상태를 바꾸는 것은 <b>처음에는 반드시 거절되고</b>, <b>무엇이 실행될지가 화면에 그대로 나온 뒤</b>에야 움직입니다. <b>승인은 한 번, 5분이면 만료됩니다.</b>',
    'howto.empty':
      '<b>설치한 그대로는 에이전트가 아무것도 할 수 없습니다.</b> 읽기 허용 목록도 비어 있고, 쓸 수 있는 디렉터리도 비어 있고, 상태를 바꾸는 작업 목록도 비어 있습니다. <b>설정을 빠뜨린 것이 아니라 그렇게 정한 것입니다.</b> 정말 필요한 만큼만 한 줄씩 넓히세요.',
    'h.tools': '에이전트가 할 수 있는 일',
    'tools.intro':
      'MCP로 <span id="tool-count"></span>개. <b>여기 있는 것은 에이전트가 실제로 읽는 설명 그대로</b>여서 영어로 둡니다. <b>없는 것</b>에 주목하세요 —— <b>임의의 명령 문자열을 받는 도구도, 내 컴퓨터로 내려받는 도구도 없습니다.</b>',
    'tools.connect': '연결하고, 어디로 보낼지 고르기',
    'tools.look': '서버 들여다보기',
    'tools.write': '쓰기 (울타리 안에서만)',
    'tools.cmds': '사람이 허용한 명령',
    'tools.console': '함께 쓰는 터미널',
    'tools.screen': '사람의 화면',
    'tools.itself': 'sshboard 자신에 대해',
    'h.state': '지금 어디까지 와 있는가',
    'state.intro':
      '<b>기능은 갖춰졌습니다. 화면은 아직 다듬어지지 않았습니다.</b> sshboard는 실제 운영 서버를 상대로 쓰이고 있고, <b>보이는 이슈 목록은 전부 거기서 나온 것</b>입니다. 다만 <b>디자인 작업은 아직 한 번도 하지 않았고, 보면 알 수 있습니다.</b> 잘 다듬어진 것을 찾고 있다면 아직 아닙니다. <b>안전 모델을 처음부터 끝까지 직접 읽고 따져볼 수 있는</b> 것을 찾는다면, 이미 읽을 수 있습니다.',
    'state.working': '되는 것',
    'state.notyet': '아직인 것',
    'state.working.list':
      '함께 쓰는 SSH 세션 하나 (GUI + 에이전트)<br />MCP 도구 35개<br />허용 목록 기반 읽기 명령<br />승인을 거친 상태 변경 작업<br />아무도 저장하지 않는 <code>sudo</code> 비밀번호<br />멈춤 버튼이 있는 공유 터미널<br />자동 업데이트',
    'state.notyet.list':
      '화면 디자인 작업<br />이 페이지의 화면 사진<br />코드 서명<br />Windows의 ssh-agent (명명된 파이프 / Pageant) —— 미확인<br />Intel macOS / Linux 빌드<br />화면 자체를 그려서 확인하는 테스트',
    'h.fence': '울타리',
    'fence.intro':
      '<b>이것은 프롬프트 안의 부탁이 아니라 코드 쪽의 제약입니다.</b> 모델이 협조하든 안 하든 그대로 작동합니다.',
    'fence.1':
      '<b><code>run_command(cmd)</code>에 해당하는 도구가 없습니다.</b> 걸러 낸 것도, 영리한 것도. <b>에이전트가 임의의 문자열을 셸에 넘길 통로 자체가 없습니다.</b>',
    'fence.2':
      '<b>읽기 명령은 사람이 쓴 목록에서 나옵니다.</b> <code>readonly.toml</code>은 <b>기본이 비어 있어서</b>, 설치한 그대로는 <b>명령 하나도 실행할 수 없습니다.</b>',
    'fence.3':
      '<b>쓰기는 연결마다 사람이 나열한 디렉터리 아래로 제한됩니다.</b> 이 목록도 <b>기본은 비어 있습니다.</b> <b>올리기와 디렉터리 만들기만</b> —— 삭제·이름 변경·이동·권한 변경·서비스 재시작·패키지 설치는 들어 있지 않습니다.',
    'fence.4':
      '<b>내 컴퓨터로 내려받는 도구가 없습니다.</b> 울타리는 <b>당신의 노트북을 전혀 지켜 주지 않으므로</b>, 그쪽으로 난 문을 에이전트에게 주지 않습니다.',
    'fence.5':
      '<b>상태를 바꾸는 작업에는 사람이 누르는 일이 필요합니다.</b> 처음 시도는 항상 거절되고, <b>무엇이 실행될지가 화면에 그대로 보입니다.</b> 승인은 <b>5분이면 만료되고 한 번분</b>입니다.',
    'fence.6':
      '<b>키는 OS 자격 증명 저장소와 ssh-agent에 있습니다.</b> sshboard는 <b>자체 키 저장소를 만들지 않습니다.</b> 연결을 나열하는 도구가 돌려주는 것은 <b>식별자뿐이며, 자격 정보는 돌려주지 않습니다.</b>',
    'h.agent': '에이전트 연결하기',
    'agent.intro':
      '실행 중인 앱을 가리키게 합니다. 이 방식은 sshboard를 <b>창 없는 중계로 띄우므로</b>, <b>토큰이 디스크에 한 번도 기록되지 않습니다</b> —— <code>~/.claude.json</code>에도, 셸 기록에도.',
    'agent.note':
      '중계는 엔진도 기록 띠도 갖지 않고 <b>스스로는 연결을 하나도 열지 않습니다</b> —— 여는 것은 앱뿐이고, 그게 핵심입니다. <b>등록은 한 번이면 됩니다.</b> 포트는 <code>22022</code>로 고정이고 토큰도 재사용하므로 <b>다시 띄워도 등록을 다시 하지 않습니다.</b> 포트가 이미 쓰이고 있으면 <b>조용히 다른 번호로 옮기지 않고</b> 화면에 그렇게 알립니다. (평범한 HTTP 방식도 있지만 그쪽은 <b>토큰이 파일에 남습니다</b> —— 창에 그것을 복사하는 버튼이 있습니다.)',
    'agent.about':
      '먼저 <code>about_sshboard</code>를 부르게 하세요. <b>이것이 무엇이고, 무엇을 해도 되고 안 되는지, 버전마다 무엇이 바뀌었는지</b>를 돌려줍니다 —— <b>사람이 설명을 붙여 넣지 않아도 되도록</b> 있는 도구입니다.',
    'h.never': '이렇게는 되지 않습니다',
    'never.1': '<b>아무도 보지 않는 동안</b> 에이전트를 서버에 붙여 돌리는 도구.',
    'never.2': '키나 패스프레이즈가 <b>저장되는</b> 곳.',
    'never.3': '<b>사람에게 보이지 않는 두 번째</b> SSH 세션을 여는 도구.',
    'f.source': '소스',
    'f.releases': '릴리스',
    'f.issues': '이슈',
  },
  ja: {
    'lede': '人と AI エージェントが、1 本の SSH を分け合う。',
    'tag.alpha': 'アルファ',
    'tag.mac': 'macOS（Apple シリコン）',
    'tag.win': 'Windows x64',
    'tag.langs': '11 言語',
    'intro':
      'たいていの道具は、AI に <code>run_command(cmd)</code> の穴を渡して、<b>人の見ていない所</b>で働かせます。sshboard は逆です。<b>AI と人が同じ 1 本の SSH を握り</b>、AI のやることは全部、<b>人が目の前にしている画面</b>に出ます。<b>見えない 2 本目は張りません。</b>',

    'h.download': '落とす',
    'dl.button': '落とす',
    'dl.mac.sub': 'Apple シリコン',
    'dl.win.sub': 'x64 ・ インストーラ（NSIS）',
    'dl.msi.title': 'Windows（MSI）',
    'dl.msi.sub': '組織で MSI 配布している場合',
    'dl.src.title': 'ソース',

    'unsigned':
      '<b>署名していません。</b>初回は Windows の SmartScreen と macOS の Gatekeeper が止めます。Windows は<i>「詳細情報」→「実行」</i>、macOS は<i>右クリック →「開く」</i>です。<b>これは鍵を扱う道具です。</b>最初に教えることが「警告を無視して開く」になるのは承知のうえで出しています —— <b>署名は、未署名で困る人が実際に出てから</b>入れます。<a href="https://github.com/meta-taro/sshboard/issues">困ったと Issue に書くこと</a>が、その判断材料になります。',

    'h.install.mac': 'macOS へ入れる',
    'install.mac':
      'macOS 版は <code>.dmg</code> ではなく <code>.app.tar.gz</code> です。<b>同じファイルを自動更新が使う</b>ためです。展開して移してください。',
    'install.mac.open': 'そのあと Finder から<b>右クリック →「開く」</b>で初回を開きます。',
    'h.install.win': 'Windows へ入れる',
    'install.win':
      '<code>-setup.exe</code> を実行します。<b>WebView2</b> が要ります。Windows 11 と最近の 10 には入っています。無ければインストーラが取りに行くので、<b>初回はインターネット接続が要ります。</b>',

    'h.ui': '画面に出ているもの',
    'ui.intro':
      '窓は 1 つ、タブは 5 つです。<b>このページにはまだ画面写真がありません。</b>この製品の画面には、必ず誰かのホスト名・利用者名・パスが写るためで、ここへ置くものは<b>架空の設定の上で撮ります。</b>それまでは、形だけ置いておきます。',
    'tab.connections': '接続',
    'tab.files': 'ファイル',
    'tab.console': '端末',
    'tab.band': '履歴',
    'tab.diag': '診断',
    'ui.left.title': '左',
    'ui.left':
      '登録したサーバーが 1 行ずつ。開くと、作業する側の上にタブとして並びます。',
    'ui.right.title': '右',
    'ui.right':
      'そのタブのもの —— ディレクトリの一覧、動いている端末、または下の 2 つの記録。',
    'ui.th.tab': 'タブ',
    'ui.th.what': '何が在るか',
    'ui.t.connections':
      '登録したサーバーと、開くボタン。<b>秘密は OS の資格情報ストアと ssh-agent に在り、ここには置きません。</b>',
    'ui.t.files':
      '2 面。片方がサーバー、もう片方が手元。上げる・落とす・ディレクトリを作る。<b>AI は人が並べたディレクトリの下だけ</b>で、<b>人は制限されません。</b>',
    'ui.t.console':
      '同じ 1 本の SSH の上の本物の端末 —— <b>AI と分け合います。</b>AI が握っている間は人の入力が締まり、［止める］が出ています。<b>見えない 2 本目は張りません。</b>',
    'ui.t.band':
      '<b>誰が何をしたか。</b>操作 1 つにつき 1 行、<code>[Human]</code> か <code>[AI]</code> の札つきで、起きた順に。<b>「AI は何をしていたのか」を読む所</b>です。',
    'ui.t.diag':
      '<b>なぜ失敗したか。</b>繋がるまでの段と、止まった理由と、次にどうするか。<b>うまくいかなかったときに読む所</b>で、<b>Issue に貼るのもここ</b>です。',
    'ui.confusable':
      '<b>「履歴」と「診断」は別のものです。</b><b>「履歴」は、やったことの記録</b> —— <i>誰が何をしたか</i>に答えます。<b>「診断」は、止まった理由の記録</b> —— <i>なぜ失敗したか</i>に答えます。<b>AI が何を走らせたかを知りたいなら「履歴」、繋がらない理由を知りたいなら「診断」</b>です。<b>この 2 つは、2026-09-18 まで「操作の記録」と「ログ」という名前でした</b> —— 実機で「どっちもログやん」と言われて変えました。',

    'h.howto': '使い方',
    'howto.1':
      '<b>サーバーを登録する。</b>［接続］で 1 件足します。識別子・名前・ホスト・利用者を書きます。<b>パスフレーズやパスワードを sshboard は保存しません</b> —— OS の資格情報ストアへ行くか、ssh-agent に任せます。',
    'howto.2':
      '<b>開く。</b>タブになります。何本でも同時に開けて、<b>その全部が画面に出ます。</b>鍵にパスフレーズが要るときは<b>問いが人の画面に出ます</b> —— <b>AI は答えられません。</b>',
    'howto.3':
      '<b>［ファイル］か［端末］で作業する。</b>どちらも同じ 1 本の上です。<b>AI のやることが別の 1 本で起きる、ということがありません。</b>',
    'howto.4':
      '<b>AI を繋ぐ</b>（下）。何か見てもらってください。<b>走らせたものは［操作の記録］にその場で出て</b>、出力は<b>人が見ているのと同じ端末</b>に出ます。',
    'howto.5':
      '<b>状態を変えるものは、人が承認する。</b>読み取りは<b>人が書いた許可リスト</b>で囲ってあります。状態を変えるものは<b>1 回目は必ず断られ</b>、<b>何が走るのかが画面にそのまま出て</b>から動きます。<b>承認は 1 回きり、5 分で切れます。</b>',
    'howto.empty':
      '<b>入れたままの状態では、AI は何 1 つできません。</b>読み取りの許可リストは空、書けるディレクトリも空、状態を変える操作の一覧も空です。<b>これは設定のし忘れではなく、そう決めてあります。</b>本当に要るぶんだけ、1 行ずつ広げてください。',

    'h.tools': 'AI にできること',
    'tools.intro':
      'MCP で <span id="tool-count"></span> 本。<b>ここに出ているのは、AI が実際に読む説明そのもの</b>なので、英語のまま出しています。<b>無いもの</b>に注目してください —— <b>任意のコマンド文字列を受け取る道具も、手元へ落とす道具もありません。</b>',
    'tools.connect': '繋ぐ・どこへ送るかを選ぶ',
    'tools.look': 'サーバーを見る',
    'tools.write': '書く（囲いの中だけ）',
    'tools.cmds': '人が許したコマンド',
    'tools.console': '分け合う端末',
    'tools.screen': '人の画面',
    'tools.itself': 'sshboard 自身のこと',

    'h.state': 'いまどこに在るか',
    'state.intro':
      '<b>機能は揃っています。画面は、まだ仕上がっていません。</b>sshboard は実運用のサーバーに対して使われていて、<b>見えている Issue は全部そこから出たもの</b>です。ただし<b>見た目の作り込みはまだ一度もしておらず、それは見れば分かります。</b>磨かれたものを探しているなら、まだです。<b>安全の仕組みを端から端まで自分で読んで、文句を言える</b>ものを探しているなら、もう読めます。',
    'state.working': '動くもの',
    'state.notyet': 'まだのもの',
    'state.working.list':
      '分け合う 1 本の SSH（画面と AI）<br />MCP 35 本<br />許可リストからの読み取りコマンド<br />承認つきの状態変更<br />誰も保存しない <code>sudo</code> のパスワード<br />［止める］付きの分け合う端末<br />自動更新',
    'state.notyet.list':
      '画面の作り込み<br />このページの画面写真<br />コード署名<br />Windows の ssh-agent（名前付きパイプ / Pageant）—— 未確認<br />Intel の macOS / Linux 版<br />画面そのものを描いて確かめるテスト',

    'h.fence': '囲い',
    'fence.intro':
      '<b>これは prompt の中のお願いではなく、コードの側の制約です。</b>AI が協力的かどうかに関係なく効きます。',
    'fence.1':
      '<b><code>run_command(cmd)</code> に当たる道具がありません。</b>絞ったものも、賢いものも。<b>AI が任意の文字列をシェルへ渡す口が、そもそも在りません。</b>',
    'fence.2':
      '<b>読み取りコマンドは、人が書いた一覧から。</b><code>readonly.toml</code> は<b>既定が空</b>なので、入れたままでは<b>1 本も走らせられません。</b>',
    'fence.3':
      '<b>書けるのは、接続ごとに人が並べたディレクトリの下だけ。</b>この一覧も<b>既定は空</b>です。<b>上げるのとディレクトリを作るのだけ</b> —— 削除・リネーム・移動・パーミッション変更・サービス再起動・パッケージ操作は入っていません。',
    'fence.4':
      '<b>手元へ落とす道具がありません。</b>囲いは<b>人の手元を 1 ミリも守らない</b>ので、そこへの扉を AI に渡しません。',
    'fence.5':
      '<b>状態を変える操作には、人が押すことが要ります。</b>1 回目は必ず断られ、<b>何が走るのかが画面にそのまま出ます。</b>承認は<b>5 分で切れ、1 回ぶん</b>です。',
    'fence.6':
      '<b>鍵は OS の資格情報ストアと ssh-agent に在ります。</b>sshboard は<b>自前の鍵ストアを持ちません。</b>接続を並べる道具が返すのは<b>識別子だけで、認証情報は返しません。</b>',

    'h.agent': 'AI を繋ぐ',
    'agent.intro':
      '動いているアプリへ向けます。この形だと sshboard が<b>窓の無い中継として起動する</b>ので、<b>合言葉がディスクに 1 度も書かれません</b> —— <code>~/.claude.json</code> にも、シェルの履歴にも。',
    'agent.note':
      '中継は実行体も帯も持たず、<b>自分では 1 本も繋ぎません</b> —— 繋ぐのはアプリだけで、そこが肝です。<b>登録は 1 回きり</b>です。口は <code>22022</code> 固定、合言葉も使い回すので、<b>起動し直しても登録し直しになりません。</b>ぶつかったときは<b>黙って別の番号へ逃げず</b>、画面にそう出ます。（平の HTTP の形もありますが、そちらは<b>合言葉がファイルに残ります</b> —— 窓にそれを写すボタンがあります。）',
    'agent.about':
      'まず <code>about_sshboard</code> を呼ばせてください。<b>これが何で、何をしてよくて何が駄目で、版ごとに何が変わったか</b>を返します —— <b>人が説明を貼らなくて済むように</b>在る道具です。',

    'h.never': 'こうはならないもの',
    'never.1': '<b>誰も見ていない所で</b>、AI をサーバーに向けて走らせる道具。',
    'never.2': '鍵やパスフレーズが<b>保存される</b>場所。',
    'never.3': '<b>人に見えない 2 本目</b>の SSH を張る道具。',

    'f.source': 'ソース',
    'f.releases': 'リリース',
    'f.issues': 'Issue',
  },
};
