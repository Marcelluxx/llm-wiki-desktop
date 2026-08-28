# LLM Wiki Desktop 0.8.2

LLM Wiki Desktop è un'applicazione per Windows che trasforma PDF, DOCX, TXT e file
Markdown in una o più knowledge base compatibili con Obsidian. L'estrazione e l'OCR
avvengono in locale; l'AI selezionata viene usata per creare concetti, entità,
sintesi, indici e collegamenti tra le note.

> **Vuoi semplicemente installare l'app?** Non usare il pulsante verde **Code** e
> non scaricare **Source code (zip)**. Vai alla pagina
> [Releases](https://github.com/Marcelluxx/llm-wiki-desktop/releases/latest), apri
> **Assets** e scarica solamente **LLM-Wiki-Setup.exe**.

## A cosa serve

Con LLM Wiki Desktop puoi:

- creare più wiki separate, ognuna con documenti, chat e impostazioni proprie;
- aggiungere più volte PDF, DOCX, TXT e MD alla stessa coda;
- estrarre la struttura dei PDF digitali e usare OCR locale sui PDF scansionati;
- evitare elaborazioni duplicate grazie alla cache basata su SHA-256;
- trasformare i documenti estratti in note Markdown organizzate per Obsidian;
- conversare con i contenuti della wiki tramite Codex, Claude Code, Antigravity,
  OpenRouter oppure Ollama;
- consultare avanzamento, download, output delle CLI ed errori senza usare un
  terminale.

I documenti originali non vengono modificati e non vengono copiati dentro la wiki.
L'app registra nel database privato della singola wiki il loro percorso e la loro
impronta SHA-256.

## Requisiti per l'utente finale

- Windows 10 o Windows 11 a 64 bit;
- almeno 8 GB di RAM; 16 GB sono consigliati per OCR e modelli locali;
- almeno 8 GB di spazio libero per applicazione, runtime, modelli e cache;
- connessione Internet durante il primo setup e per i provider cloud;
- un account o una chiave valida per il provider AI scelto.

Python, Java, Node.js e Rust **non devono essere installati manualmente**. Il setup
include i runtime Python e Java privati dell'app. Quando scegli una CLI AI, l'app
chiede conferma e prepara anche un runtime Node.js privato, senza modificare quello
eventualmente presente nel computer.

Il setup include anche l'inventario delle licenze delle dipendenze in un percorso
compatto del runtime: non è necessario scaricare pacchetti aggiuntivi o ricostruire
l'ambiente a mano.

Obsidian è facoltativo: le wiki sono normali cartelle Markdown e restano leggibili
anche senza Obsidian.

## Installazione semplice da GitHub

1. Apri la pagina
   [Ultima Release](https://github.com/Marcelluxx/llm-wiki-desktop/releases/latest).
2. Scorri fino alla sezione **Assets**.
3. Scarica **LLM-Wiki-Setup.exe**.
4. Facoltativo ma consigliato: scarica anche **SHA256SUMS.txt** e verifica il file
   seguendo la sezione successiva.
5. Fai doppio clic su **LLM-Wiki-Setup.exe**.
6. Se Windows chiede l'autorizzazione all'installazione, conferma solamente dopo
   avere verificato che il file provenga da questo repository.
7. Completa il setup e avvia **LLM Wiki Desktop** dal menu Start.

Non estrarre né eseguire i file **Source code.zip** o **Source code.tar.gz**: GitHub
li crea automaticamente per gli sviluppatori e non contengono l'installer pronto.

### Avviso Microsoft Defender SmartScreen

Finché il progetto non dispone di un certificato di firma Windows, SmartScreen può
mostrare **Windows ha protetto il PC** anche se il file è quello corretto. In questo
caso:

1. controlla che l'indirizzo inizi con
   `https://github.com/Marcelluxx/llm-wiki-desktop/`;
2. verifica il checksum SHA-256;
3. seleziona **Ulteriori informazioni** e poi **Esegui comunque** solo se i due
   controlli precedenti sono corretti.

Non disattivare l'antivirus e non scaricare copie dell'app da altri siti.

## Verifica del download

Apri PowerShell nella cartella Download ed esegui:

```powershell
Get-FileHash -Algorithm SHA256 .\LLM-Wiki-Setup.exe
```

Il valore mostrato deve essere identico a quello associato a
`LLM-Wiki-Setup.exe` dentro **SHA256SUMS.txt**. Se è diverso, elimina il file e
scaricalo nuovamente dalla Release ufficiale.

## Primo avvio

### 1. Scegli la lingua

Seleziona Italiano o English. Potrai cambiarla successivamente dalle impostazioni.

### 2. Scegli il provider AI

Puoi configurarlo subito oppure entrare nell'app e farlo più tardi dalla sezione
**AI provider**.

| Provider | Cosa serve | Dove lavora il modello |
| --- | --- | --- |
| Codex | Account supportato e accesso tramite CLI ufficiale | Cloud OpenAI |
| Claude Code | Account o credenziali supportate dalla CLI ufficiale | Cloud Anthropic |
| Antigravity | Account supportato dalla CLI ufficiale | Cloud del provider |
| OpenRouter | API key OpenRouter e scelta del modello | Cloud del modello scelto |
| Ollama | Installazione guidata e almeno un modello locale | Sul computer |

Per Codex, Claude Code e Antigravity l'app controlla la presenza della CLI. Se
manca, mostra cosa verrà scaricato e richiede una conferma esplicita. Le schermate di
accesso sono quelle ufficiali del provider; LLM Wiki Desktop non legge né copia le
password o i token delle CLI.

La chiave OpenRouter viene inserita nella finestra **Configura**, rimane nascosta e
viene conservata tramite Windows Credential Manager. Ollama può richiedere diversi
GB aggiuntivi a seconda del modello scelto.

L'utilizzo dei provider cloud può essere soggetto a limiti o costi stabiliti dal
provider. LLM Wiki Desktop non include crediti AI.

### 3. Crea la prima wiki

1. Premi **Crea wiki**.
2. Assegna un nome riconoscibile, per esempio “Università” o “Documentazione”.
3. Seleziona una cartella dedicata. Non scegliere la radice del disco o l'intera
   cartella utente.
4. Entra nella wiki e premi **Add documents**.
5. Seleziona uno o più file PDF, DOCX, TXT o MD.
6. Premi **Start import** e attendi la conferma di completamento.
7. Quando gli artefatti sono validi, premi **Ingest** per creare o aggiornare la
   knowledge base secondo il file `AGENTS.md` della wiki.

Puoi aggiungere altri documenti in seguito: la nuova selezione viene aggiunta alla
coda e i duplicati già conosciuti vengono recuperati dalla cache quando possibile.

## OCR, CPU e schede NVIDIA

- I PDF con testo selezionabile usano l'estrazione strutturale veloce di
  OpenDataLoader PDF.
- I PDF scansionati o senza testo utilizzabile passano attraverso OCR e analisi del
  layout in locale.
- Al primo OCR possono essere scaricati modelli aggiuntivi. L'app mostra
  avanzamento, percentuale e dettagli disponibili.
- In assenza di una scheda NVIDIA l'app usa la CPU e non scarica il pacchetto CUDA
  opzionale.
- Con una GPU NVIDIA compatibile puoi aprire **Impostazioni > Prestazioni** e
  abilitare esplicitamente l'accelerazione. Il download può richiedere circa
  1,8 GB aggiuntivi.

I primi documenti scansionati sono normalmente più lenti perché devono inizializzare
il motore e i modelli. I PDF successivi dello stesso gruppo riutilizzano il backend
già caricato.

## File creati nella wiki

La parte leggibile contiene normalmente:

```text
index.md
sources/
concepts/
entities/
syntheses/
indexes/
attachments/
AGENTS.md
```

Lo stato tecnico viene conservato in `.llm-wiki/`: database, cache, artefatti,
staging e log. L'app aggiunge questa cartella alle esclusioni di Obsidian, così i
file OCR e le immagini tecniche non riempiono il grafo. Non cancellare manualmente
`.llm-wiki/` se vuoi conservare cache, cronologia e possibilità di recupero.

## Privacy

- Selezione, hashing, estrazione e OCR avvengono localmente.
- Quando usi Codex, Claude Code, Antigravity o OpenRouter, il contenuto necessario
  alla richiesta viene inviato al provider selezionato.
- Con Ollama il modello può funzionare interamente in locale.
- Le credenziali non vengono scritte nella wiki o nei log dell'app.
- I documenti originali non vengono modificati.
- Ogni wiki è isolata: la chat e l'ingest ricevono solamente il contesto della wiki
  attiva.

Prima di caricare documenti riservati, verifica sempre condizioni, privacy e costi
del provider scelto.

## Risoluzione dei problemi

### Il pulsante Import non parte

- Controlla che almeno un file supportato sia ancora presente nella coda.
- Apri **Show processing logs** e cerca la prima riga rossa.
- Verifica che il file originale non sia stato spostato, rinominato o cancellato.
- Premi **Stop import** se un lavoro precedente risulta ancora attivo, poi riprova.

### L'OCR sembra fermo

Il primo avvio del modello può richiedere più tempo. Il pannello attività mostra
tempo trascorso, CPU, RAM e download. Se non compare alcuna attività per diversi
minuti, interrompi il lavoro, apri i log e allega il relativo codice errore alla
segnalazione.

### Il provider non risponde

Apri **AI provider**, premi **Aggiorna stato** e controlla:

- che il provider selezionato mostri **Connesso**;
- che l'accesso ufficiale sia stato completato;
- che la connessione Internet sia disponibile;
- che la chiave OpenRouter non sia scaduta;
- che Ollama sia avviato e abbia un modello selezionato.

La chat mostra il flusso CLI dettagliato e salva una diagnostica redatta in
`.llm-wiki/logs/provider-events.jsonl`.

### Le immagini OCR compaiono nel grafo Obsidian

Apri almeno una volta la wiki con LLM Wiki Desktop aggiornato, poi riavvia Obsidian
o usa il comando **Reload app**. L'app configura automaticamente l'esclusione di
`.llm-wiki/` e disabilita gli allegati nel grafo, preservando le altre impostazioni
del vault.

### Come segnalare un bug

Apri una [nuova issue](https://github.com/Marcelluxx/llm-wiki-desktop/issues/new)
e indica:

- versione dell'app e versione di Windows;
- provider selezionato;
- formato e numero dei documenti, senza allegare contenuti riservati;
- fase in cui si verifica il problema;
- messaggio mostrato dall'app;
- log redatti pertinenti.

Non pubblicare API key, token, password, documenti personali o l'intera cartella
`.llm-wiki`.

## Disinstallazione e aggiornamenti

Puoi disinstallare LLM Wiki Desktop dalle normali impostazioni **App installate** di
Windows. La disinstallazione non cancella le cartelle delle wiki e non rimuove le
credenziali possedute direttamente dalle CLI dei provider.

Per aggiornare, scarica il nuovo `LLM-Wiki-Setup.exe` dalla Release più recente,
chiudi l'app e avvia il setup. Prima di aggiornare una versione importante è sempre
consigliato eseguire una copia della propria cartella wiki.

## Stato della versione 0.8.2

La 0.8.2 è la prima release pubblica Windows x64. La pipeline genera inizialmente
una **bozza di Release**: viene pubblicata solamente dopo un test manuale di
installazione, avvio, import e disinstallazione su un PC pulito.

Limitazioni note:

- Windows ARM, macOS e Linux non sono ancora distribuiti;
- senza un certificato di code signing può comparire SmartScreen;
- account, disponibilità, modelli, quote e costi dei provider dipendono dai relativi
  servizi;
- il primo download dei modelli OCR o Ollama può essere voluminoso;
- l'accelerazione NVIDIA rimane un componente opzionale separato.

## Per sviluppatori

Questa sezione non serve per installare l'app dalla Release.

### Requisiti di sviluppo

- Windows 10/11 x64;
- Node.js 24;
- Rust 1.98.0 con toolchain MSVC;
- Python da 3.12 a 3.14;
- Java 21;
- `uv`, Microsoft C++ Build Tools e WebView2.

Setup isolato:

```powershell
.\scripts\bootstrap-dev.ps1
.\scripts\quality.ps1
```

Avvio in sviluppo:

```powershell
npm run tauri -- dev
```

Build locale senza installer:

```powershell
npm run tauri -- build --debug --no-bundle
```

Il bootstrap mantiene Rust e le cache del progetto sotto `.tools/` e crea
l'ambiente Python sotto `.venv/`; entrambe le cartelle sono escluse da Git.

### Creazione della Release 0.8.2

1. Esegui `npm run version:check` e `.\scripts\quality.ps1`.
2. Crea e invia il tag `v0.8.2`.
3. Il workflow **Release Windows** prepara runtime privati, installer e checksum.
4. GitHub crea una bozza della Release con `LLM-Wiki-Setup.exe` e
   `SHA256SUMS.txt`.
5. Scarica la bozza e completa lo smoke test su un PC Windows pulito.
6. Pubblica la bozza solamente se installazione, avvio, import e disinstallazione
   hanno esito positivo.

La firma digitale Windows è applicabile quando il proprietario del progetto
fornisce un certificato di code signing e configura i relativi secret GitHub. Le
credenziali di firma non devono mai essere inserite nel repository.

## Architettura

- Tauri 2 e React/TypeScript per l'interfaccia;
- Rust per orchestrazione, sicurezza dei percorsi e processi;
- worker Python privato per estrazione e OCR;
- OpenDataLoader PDF 2.5.5;
- SQLite per catalogo, job, chat e cache;
- contratti JSON Schema versionati;
- adapter separati per ogni provider AI.

La progettazione completa è disponibile in
[`docs/superpowers/specs/2026-08-25-llm-wiki-desktop-design.md`](docs/superpowers/specs/2026-08-25-llm-wiki-desktop-design.md).
