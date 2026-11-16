# The port checklist

Each iPad view file (kiff/tafs `ios/Tafs/Views`, at the pin in `shared-pin.json`) against its
Linux status. A file is **ported** when every branch it draws is on Linux, **partial** when part
of it is (what's missing is named), **not started**, or **not for Linux** (and why). The PR that
ports a file updates its line here.

## Ported

| iPad file | Lines | What | Models | Linux |
| :- | -: | :- | :- | :- |
| OnboardingView.swift | 63 | The welcome: what tafs is, Sign in with KIFF, Try it with sample data | AppModel.startSample | `welcome.rs` |
| AIConsentView.swift | 74 | The AI-data consent screen and Account's row | `AIConsent`, `allowAI`, `notNowAI`, `withdrawAI`, `loadAIConsent` | `ai_consent.rs`, `tafs-core/ai_consent.rs`: the screen after sign-in and from Account, Allow with the words' version (a 409 shows the newer words), Not now for the session, Withdraw; a refused model turn has none to ask on Linux (no chat yet) |
| ApprovalSetupView.swift | 102 | Why this device can't approve, in an approval's place: `ApprovalGate`, `ApprovalNotice`, `ApprovalNotReady` | `ApprovalReadiness` (Biometry.swift): every state's words, `approvalBlocked` | `approval.rs`, `tafs-core/approval.rs`: this computer is `Unsupported` (no Face ID or Touch ID), so its line alone; `ReadinessReader` reads a value that never changes here, and `openTafsSettings` opens iOS settings |
| DeleteAccountView.swift | 60 | Deleting the account: the warning, What goes, Delete account, Cancel | `deleteAccount` (signed: not on Linux) | `kiff_account.rs` › `delete_sheet`, `tafs-core/account.rs` › `delete_lines`; Delete account is `ApprovalGate`'s line, since this computer can't confirm, and the footer naming Face ID goes with it |
| ExportChatsView.swift | 82 | Export my chats: preparing, the files, Ready… Share again, Try again | `exportChats`, `splitExport` | `kiff_account.rs` › `export_section`, `tafs-core/export_chats.rs`; saved in a folder the person picks, as Linux has no share sheet |
| PairSignInView.swift | 104 | Signing in from another phone: the QR, the check code, the wait, the claim | KIFF/Pairing (§5.8) | The sign-in's "Sign in with your other phone" opens this computer's pairing (`pairing.rs`, `tafs-core/pairing.rs`, from tafs-macos): the same QR, check code, wait and claim, with a computer's key (`passkey` kind, software key store) in a phone's place |
| RecordSearchViews.swift | 192 | The Record's search, its cases and Export receipts | `searchRecord`, `recordCase`, `exportReceipts` | `RecordSearchBox`, `RecordResults`, `RecordHitRow` (a taf not on Home as its plain panel), `RecordCaseSheet` (`record_search.rs`, `tafs-core/record_search.rs`); `ExportReceiptsRow`, the files saved in a folder the person picks (`receipts.rs`, `tafs-core/receipts.rs`) |

## Partial

| iPad file | Lines | What | Linux has | Missing |
| :- | -: | :- | :- | :- |
| SignInView.swift | 333 | `SignInView`: Sign in with KIFF. Also `SettingsView`, `PlanRow`, `AccountView`, `AccountLines` (Account) | `SignInView` whole: the code, Open KIFF, the link and Copy, the wait, the problem and Try again, the terms line (`sign_in.rs`, `tafs-core/signin.rs`). `SettingsView`'s sections in order, with Coach's Show suggestions and the AI-data consent's row (`ai_consent.rs`), the theme, sign out with its question, Clear the sample, `AccountLines`, `PlanRow`'s title; `AccountView`'s lines (`kiff_account.rs`) (`account.rs`, `tafs-core/account.rs`) | `AccountView`'s using this computer's zone (dimmed; Export my chats and Delete account are their own files, ported); Change language, dimmed (the in-app picker is S24, after the iPad's) |
| ShareViews.swift | 405 | Sharing a taf (`ShareSheet`, `SharedWithLine` and the rest), and the Overview (`OverviewView`) | `OverviewView` whole: the title, NEEDS YOU and what else waits, the grid with `cardLine`, `lastResult` from each taf's last run (`loadReport`) and `usedBy` from its shares (`overview.rs`, `tafs-core/overview.rs`, `card.rs`, `tafs-core/access.rs`); its order (`listedTafs`, `sidebar.rs` › `by_recent`) | Sharing itself (`ShareSheet`, `SharedWithLine`) |
| NeedsYouView.swift | 603 | Needs you: held calls, "Did this happen?", steps, Coach's waits, knocks, alerts | `NeedsYouView`'s held calls and its empty state; `HeldCallCard`'s face, countdown, taf's words, details, expired and someone-answers states, and Answer in KIFF; `CoachWaitingCard`'s line and `SetAllAsideButton`, their acts dimmed; `DidThisHappenCard` whole for a computer that can't approve: KIFF's words or title and amount, why, and `ApprovalGate`'s can't-approve line in place of No and Yes; `CardUnitExceptionPanel`'s words, its Coach button for whoever may use Coach and manages the taf, and Dismiss, both dimmed; knocks, each opening its taf's Activity (`needs_you.rs`, `tafs-core/needs_you.rs`, `held.rs`, `unproven.rs`) | Answering here (approve, extra room, next steps, refuse, Allow from now on, install); `AlertsCard` (only for a session with an approval key: never Linux's yet); setting Coach's chats aside; discussing or dismissing a Card unit request; scrolling to a call |
| AccountPreferencesView.swift | 37 | Account's theme and time zone pickers | `ThemePicker` with `ThemePreference`, kept across launches (`account.rs`, `store.rs`); `TimeZonePicker`'s search and list with the account's zone checked (`zones.rs`, `tafs-core/zones.rs`) | Choosing a zone and using this computer's (signed: dimmed) |
| RecordView.swift | 689 | The Record, its rows, filters and receipts | `RecordView`'s list: the taf chips, the filters, days with what was spent, folded reads, `RecordRow`'s face and lines (`record.rs`, `tafs-core/record.rs`); the search box and Export receipts under the lead (`RecordSearchViews`); `ReceiptView`, a line's receipt read and checked (`receipt_sheet.rs`, `tafs-core/receipt_view.rs`, `tafs-core/receipt_check.rs`) and its "KIFF receipt" mark; what a line acted on (`naming`, `tafs-core/record.rs`) | which vendor refused a call, by its connection (`vendors`) |
| TafDetailView.swift | 1467 | A taf's page: head, tabs, Card bar, Activity, Card, Setup, panels | `header`, `StateChip`, the tabs, `CardBar` (it opens Card), the held calls, the sample's `activity(_:)` with `ActivityFilter`, `TafActivityList` for a tafs cloud taf, the Card tab's `CardStack`, `WalletLayout`, `TafCardFace`, `CardLines` (`taf.rs`, `tafs-core/taf_page.rs`, `cards.rs`); `WorkPanel` and `HealthLines` read only (`work_panel.rs`, `tafs-core/work.rs`); on Card, tafs cloud's time zone lines, `eachItem`, `CountedView`, pending and the unit words (`taf.rs`, `tafs-core/cards.rs`); `RunsPanel` with Run now (`runs_panel.rs`, `tafs-core/runs.rs`) | Chat tab (dimmed); on Setup: Starts when, Results go to, `ConnectedToolsRing`, its Connect again, Improve in Coach, the sample's Setup; on Card: Connect again (Coach), `HaltRow`, Versions, Usage, Revoke's action; `headerLine`, the side column, Remove's action; `WorkPanel`'s Answer within and Send again; `ConnectionsPanel`, `VersionsPanel`; the Setup tab of a tafs cloud taf: its Card unit requests, the job, `MemorySummaryPanel` and `MemoryPanel` with Forget, Remove for good dimmed (`setup_pane.rs`, `tafs-core/setup.rs`) |
| TafPresentationViews.swift | 492 | A taf's Activity list and its rows, from tafs cloud; its runs and work | `TafActivityList`'s held calls, its work (`WorkPanel`), its runs (`RetainedRuns` and every run's sheet), filters, groups, pages and foot; `SourceActivityRow`; `NextStepRow` as its words (`taf.rs`, `tafs-core/activity.rs`); `TafKnocksPanel` with `KnockCard`'s messages and `DoItSheet`, answered at tafs cloud (`knocks.rs`, `tafs-core/doors.rs`); its Card unit requests and notice (`.cardUnits`, from Needs you's panel) | the next step's buttons; a row's receipt |
| UsageView.swift | 348 | Plan & usage: the balance, buying, who bills, members' requests | `UsageView`'s title, loading and order; `BalancePanel`'s edge, words, `Meter`, detail, warning and why it couldn't be read; the Credits panel shown or left out; KIFF's usage read with `billingPanel`'s lines, `askPanel` and `requestsPanel` with their buttons dimmed, Raise behind `ApprovalGate`, and why KIFF couldn't be read (`usage.rs`, `tafs-core/usage.rs`) | Raise the limit itself (signed), declining and the read after it; Ask an admin (`AskAdminView`) and Withdraw; buying packs and Solo, and approving a pack (the store); Manage and Cancel Solo (Apple's page); `planMessage`; refreshing in place |
| CreditsViews.swift | 181 | The credits the account's tafs used, and one taf's runs | `CreditsPanel` whole: both periods, its errors, each taf's row (opening its runs at KIFF), Coach, not counted, the note; `TafCreditsView` with More runs; `CreditsRunRow`'s words and time (`usage.rs`, `tafs-core/usage.rs`) | Pull to refresh (Try again instead) |
| ConnectAppsView.swift | 1128 | Connect apps, a connected app's detail, connecting's approval | `ConnectAppsView`'s list: the search, Yours and Discover (`split`, `keys`), the table (`ConnectColumns`) with each row (`ConnectRow`), "Make one first", More apps, Not listed? Ask Coach; each app's logo from tafs cloud; the narrow window's one-line rows (`second`) (`connect.rs`, `tafs-core/connect.rs`) | Connecting (dimmed: the approval is signed), "Which taf?", `ConnectApprovalSheet`; `AppConnectionsView`, `ConnectionDetailView`, Disconnect and Clear |
| DevicesView.swift | 69 | Account › Devices | The list with each device's name, last use (`lastUsed`, iOS's relative words) and This device, the note, KIFF's error; the sample's three (`devices.rs`, `tafs-core/devices.rs`) | Signing another device out (an approval: dimmed); Add a device (a phone pairs a computer: not for Linux) |
| CalendarView.swift | 434 | Calendar: Day, Week, Month | Both layouts by `CalendarLayout.of`: the header with Today, previous and next, the title, the date picker and the views; the navigator, the month's cells, the strip, the time slots, the day's list and the agenda; tafs cloud's halted and "more" words; opening a taf (`calendar.rs`, `tafs-core/calendar.rs`) | Pull to refresh |
| PeopleViews.swift | 291 | People & groups, naming a group, adding someone | `PeopleGroupsView`'s list: the account's line, each group's name, `uses`, people `managersFirst` with `PersonBadge`, `managesGroup`, the 20-second read (`people.rs`, `tafs-core/people.rs`) | Every change, signed: + Group, rename, delete, Add someone (dimmed), remove and change a role; `GroupNameSheet`, `AddToGroupSheet` |
| YourStepCard.swift | 46 | A step a taf handed its owner | The card whole, with the vouched link opening in the browser (`needs_you.rs`, `tafs-core/waits.rs`) | Done and Skip it (signed: dimmed) |
| HomeRecipes.swift | 242 | Home with no taf: the recipes | `HomeRecipes` whole: tafs cloud's line, the category chips in their colours narrowing the list, each category's recipes (`RecipeRow`: headline, line, apps with their logos from tafs cloud, the colour's edge); `OwnWordsRow`; `RecipeSheet` whole; `TafsCloud.Recipes` read as its decoders (`recipes.rs`, `tafs-core/recipes.rs`) | Coach, which "Say it in your own words" and "Hand it to Coach" open (both there, dimmed) |
| RecipeGallery.swift | 135 | The jobs' gallery | `RecipeGallerySheet` browsing (title, "All jobs", every job, each sheet read only with "Try it with Coach"), `ExploreRow`, `RecipeStrip` with See all jobs (`recipes.rs`, `shell.rs`, `tafs-core/recipes.rs`) | `.create` from + New, with its own-words row: with Coach |
| HaltScreen.swift | 176 | A halt of one taf or every taf | `HaltScreen` whole: halting (one tap, unsigned), Halting… with its parts and Halt again, read every 2 s; `HaltRow` framed on a taf's Card; Halt all in the sidebar (`halt.rs`, `tafs-core/halt.rs`) | Resuming (signed by the device: there, dimmed, with why); Siri's tip (not for Linux) |
| HomeSplitView.swift | 584 | The sidebar for the whole app and what it opens | The brand, the screens with Needs you's count, NEEDS YOU's held calls, Your tafs relevant now, Shared with you and Other agents with Cards, the plan's line (`PlanLine`), Coach and Account at the foot, the chosen row's fill (`shell.rs`, `tafs-core/sidebar.rs`); Home read from tafs cloud; the recipes strip and Explore with its gallery (`recipes.rs`); the halt-all row (`halt.rs`); tafs being set up among Your tafs with Finish's line, Discard there dimmed, and Other agents only when Home sends no `setting_up` (`tafs-core/home.rs` › `SettingUp`) | What each row opens (title only until each screen is ported); Finish setting up and + New, both with Coach; Discard itself (signed); the Unreachable line at the foot |
| NotificationSettingsView.swift | 138 | Account › Alerts | The two example alerts (`LockScreenAlert`); signed in, Alerts on this computer and Sound, kept across launches and followed by the held calls' watch; the line under them (`alerts.rs`, `tafs-core/alerts.rs`, `store.rs`) | What alerts you and `AlertingTafsView` (a phone's push choices, read with its push token, which a computer hasn't: the iPad draws none of it there either); Haptics (a phone's) |
| TafDialPanel.swift | 228 | The dial's requests and confirmation, and a run's own page | `KnobRequestDetail`: where it came from, its line, its steps with their marks, Try the reply now for whoever manages the taf (`runs_panel.rs`, `tafs-core/runs.rs`) | The dial's requests and their confirmation; Ask Coach (Coach isn't on this computer) |
| DoorsView.swift | 923 | A taf's Doors tab | `KnockCard` for a message and `DoItSheet`, on a taf's Activity (`knocks.rs`, `tafs-core/doors.rs`) | The Doors tab itself: its doors, the dial, adding and closing one, pairing and keys; the day's line (`rest`) and its OK |
| DiscardSettingUp.swift | 35 | Discard a taf being set up | `DiscardSettingUpButton`'s button in tafs cloud's words, beside the taf, for the person whose setup it was (`shell.rs`) | Its confirmation and the discard: signed, so the button is there, dimmed |

## Not started

| iPad file | Lines | What | Models |
| :- | -: | :- | :- |
| AllowSheetView.swift | 96 | "Allow this from now on" | HoldAllow |
| AppEventApprovalSheet.swift | 54 | An app event's filters before signing a subscription | |
| AttachViews.swift | 267 | Attaching photos and files in a chat | |
| CoachMode.swift | 131 | Coach mode over the app | |
| CoachToolsStep.swift | 255 | Coach's tools still to give | |
| DriveViews.swift | 73 | Coach asks to drive the app | |
| MicButton.swift | 122 | The mic in a chat's message bar | |
| PlanViews.swift | 268 | Buying credits (held for the owner's money question) | |
| QuestionsCard.swift | 121 | Several questions at once | QuestionAnswers |
| ReportAnswerSheet.swift | 124 | Report an answer | |
| SetupChatView.swift | 1946 | Coach's conversation | SetupComponent, SetupShape, SentMessages |
| SetupShapePanel.swift | 130 | What Coach has made so far | SetupShape |
| TafChatView.swift | 826 | A taf's chat | |
| TafKnobView.swift | 377 | The taf's remote | KnobNavigation |
| TafSetupViews.swift | 512 | A taf's Setup | |
| TrialViews.swift | 214 | Trials of a new version | |
| WorkLog.swift | 181 | What a taf does while it answers | |

## Not for Linux

| iPad file | Lines | Why |
| :- | -: | :- |
| HomeView.swift | 403 | The phone's Home. A wide screen uses HomeSplitView. |
| TafsBar.swift | 121 | The phone's bar along the foot. A wide screen has the sidebar. |
| PairComputerView.swift | 222 | The phone's side of pairing: the camera reads a computer's code. The computer's side is tafs-macos's, ported in [pairing](pairing.md) (`pairing.rs`, `tafs-core/pairing.rs`). |
| TafKnobRemote.swift | 64 | Keeps the remote above the phone's tab bar. |

## Shared pieces

| iPad file | Linux |
| :- | :- |
| Design/Theme.swift | `tafs-core/theme.rs`, `style.rs`: every colour, light and dark, the brand sweep and the taf colours |
| Design/Components.swift | `widgets.rs`: `TafAvatar`, `Eyebrow`, `PrimaryLook`, `OutlineLook`, `Panel`, `PanelLink`, `Legal`, `shortTime`; `Meter` (`usage.rs`); `AccountTime`, `zonedShortTime`, `endsWords`'s zone (`tafs-core/account_time.rs`). Not yet: `endsWords`'s abbreviated date (it's ISO), `StateChip`, `DoorRim`, `UnreachableLine` and the rest |
| KIFF/KIFFClient.swift | `tafs-core/signin.rs`, `kiff.rs`: `DeviceStart`, `DeviceToken`, `startDeviceSignIn`, `pollDeviceToken`, `signOut`, `fingerprint`, the refusal's reading, `acceptLanguage`, `screenLanguages` |
| KIFF/KIFFClient.swift (activity) | `ActivityCall`, `activity()` (`record.rs`, `kiff::activity`) |
| KIFF/KIFFMapping.swift (record) | `record`, `executed`, `counted`, `isCurrency`, `recordID` (`record.rs`) |
| KIFF/KIFFClient.swift (account) | `Account`, `me()`, `signOut` (`account.rs`, `kiff::me`) |
| Design/Components.swift (sign out) | `SignOutWay.signOut`, `confirmingSignOut` (`main.rs`) |
| Model/ThemePreference.swift | `ThemePreference` (`account.rs`, `store.rs`) |
| KIFF/PlainError.swift | `signin::plain_words` |
| KIFF/TafsCloud.swift | `Home`, `HomeTaf`, `NeedsYou`, `home()` (`tafs-core/home.rs`, `kiff::home`) |
| KIFF/KIFFClient.swift (usage) | `Usage` (`Billing`, `Store`, `Pack`, `offered`), `PackRequest`, `usage()`; `managesBilling` (`tafs-core/usage.rs`, `kiff::usage`) |
| KIFF/TafsCloud+Knob.swift | `ConnectApps`, `App`, `connectApps(_:provider:cursor:)`, `loadMoreConnectApps`'s merge (`tafs-core/connect.rs`) |
| Design/Words.swift | `Words.list` (`words::list`) |
| KIFF/KIFFClient.swift (access) | `Access`, `myAccess()`, `Share`, `shares(agent:)`; `AppModel.manages`, `issuer(of:)` (`tafs-core/access.rs`, `main.rs`) |
| KIFF/TafsCloud.swift (runs) | `Run`, `Report`, `runs(_:)`, `report(_:session:)`; `AppModel.loadReport` (`tafs-core/access.rs`, `main.rs`) |
| KIFF/KIFFClient.swift (people) | `Group`, `GroupMember`, `Person`, `groups()`, `people()` (`tafs-core/people.rs`, `kiff::kiff_get`) |
| KIFF/KIFFClient.swift (devices) | `Device`, `devices()` (`tafs-core/devices.rs`, `kiff::devices`) |
| App/SampleScreens.swift | `devices(now:)` (`devices::sample`) |
| KIFF/TafsCloud.swift (usage) | `Balance`, `Credits`, `TafCredits`, `LenientItem`, `balance()`, `credits(previous:)`, `credits(taf:previous:after:)` (`tafs-core/usage.rs`, `kiff::cloud`) |
| App/AppModel.swift (usage) | `refreshUsage`'s tafs cloud reads, `readPreviousCredits`, `readTafCredits`, `creditsWords`, `showsCreditsPanel`, `sampleBalance`, `sampleCredits` (`usage.rs`) |
| KIFF/TafsCloud+Work.swift | `Health`, `TafWork.Summary`, read leniently (`home.rs`) |
| KIFF/KIFFMapping.swift | `taf`, `money`, `window` (`home.rs`); `held`, `says`, `toolWords`, `name`, `reasonBreach`, `moneyReason`, `countReason`, `ordinal`, `details`, `amount`, `tafWords`, `outcomeText` (`held.rs`) |
| KIFF/KIFFClient.swift (held) | `CardException`, `heldCalls`, `answerPage` (`held.rs`, `needs_you.rs`, `kiff::held`) |
| App/AppModel.swift | `startSignIn`, `finishSignIn`, `transient`, `relevantNow`, `needsYouCount`, `accountLabel`, `coachAvailable`, `propose`'s held call (the sample's) |
| Services/ThisDevice.swift | `signin::PLATFORM`, `KIFF_KIND` |
| Services/TafTemplates.swift | `tafs-core/sample.rs`: the five sample tafs and their held calls |
| Model/Models.swift | `Money.figure` for whole euros (`money.rs`); `Window`, `TafCard.hero` (`card.rs`) |

## Coverage

Ported 7, partial 22, not started 17, not for Linux 4: the 50 view files at the pin.
