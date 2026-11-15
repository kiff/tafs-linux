# The port checklist

Each iPad view file (kiff/tafs `ios/Tafs/Views`, at the pin in `shared-pin.json`) against its
Linux status. A file is **ported** when every branch it draws is on Linux, **partial** when part
of it is (what's missing is named), **not started**, or **not for Linux** (and why). The PR that
ports a file updates its line here.

## Ported

| iPad file | Lines | What | Models | Linux |
| :- | -: | :- | :- | :- |
| OnboardingView.swift | 63 | The welcome: what tafs is, Sign in with KIFF, Try it with sample data | AppModel.startSample | `welcome.rs` |

## Partial

| iPad file | Lines | What | Linux has | Missing |
| :- | -: | :- | :- | :- |
| SignInView.swift | 333 | `SignInView`: Sign in with KIFF. Also `SettingsView`, `PlanRow`, `AccountView`, `AccountLines` (Account) | `SignInView` whole: the code, Open KIFF, the link and Copy, the wait, the problem and Try again, the terms line (`sign_in.rs`, `tafs-core/signin.rs`) | "Sign in with your other phone" is there, dimmed: it opens `PairSignInView`. Account's screens (`SettingsView` and the rest) |
| ShareViews.swift | 405 | Sharing a taf (`ShareSheet`, `SharedWithLine` and the rest), and the Overview (`OverviewView`) | `OverviewView` whole: the title, NEEDS YOU and what else waits, the grid with `cardLine` and `lastResult` (`overview.rs`, `tafs-core/overview.rs`, `card.rs`) | The Overview's reads: who uses each taf (`shares`) and each taf's report (`loadReport`), with Home from tafs cloud. Sharing itself |
| NeedsYouView.swift | 603 | Needs you: held calls, "Did this happen?", steps, Coach's waits, knocks, alerts | `NeedsYouView`'s held calls and its empty state; `HeldCallCard`'s face, countdown, taf's words, details, expired and someone-answers states, and Answer in KIFF (`needs_you.rs`, `tafs-core/needs_you.rs`, `held.rs`) | Answering here (approve, extra room, next steps, refuse, Allow from now on, install); `DidThisHappenCard`, `AlertsCard`, `CoachWaitingCard`, `SetAllAsideButton`; knocks; scrolling to a call |
| HomeSplitView.swift | 584 | The sidebar for the whole app and what it opens | The brand, the screens with Needs you's count, NEEDS YOU's held calls, Your tafs relevant now, Coach and Account at the foot, the chosen row's fill (`shell.rs`, `tafs-core/sidebar.rs`) | What each row opens (title only until each screen is ported); Shared with you; Other agents; tafs being set up and Discard; the recipes strip and Explore; + New and its gallery; the halt-all row (sidebar without `tab`); Unreachable and Plan lines at the foot; Home read from tafs cloud |

## Not started

| iPad file | Lines | What | Models |
| :- | -: | :- | :- |
| AIConsentView.swift | 74 | The AI-data consent screen | |
| AccountPreferencesView.swift | 37 | Account's theme and time zone pickers | ThemePreference |
| AllowSheetView.swift | 96 | "Allow this from now on" | HoldAllow |
| AppEventApprovalSheet.swift | 54 | An app event's filters before signing a subscription | |
| ApprovalSetupView.swift | 102 | This device's approval readiness | |
| AttachViews.swift | 267 | Attaching photos and files in a chat | |
| CalendarView.swift | 434 | Calendar: Day, Week, Month | CalendarModel |
| CoachMode.swift | 131 | Coach mode over the app | |
| CoachToolsStep.swift | 255 | Coach's tools still to give | |
| ConnectAppsView.swift | 1128 | Connect apps | |
| CreditsViews.swift | 181 | The credits the account's tafs used | |
| DeleteAccountView.swift | 60 | Deleting the account | |
| DevicesView.swift | 69 | Account › Devices | |
| DiscardSettingUp.swift | 35 | Discard a taf being set up | |
| DoorsView.swift | 923 | A taf's Doors tab | |
| DriveViews.swift | 73 | Coach asks to drive the app | |
| ExportChatsView.swift | 82 | Export my chats | |
| HaltScreen.swift | 176 | A halt of one taf or every taf | |
| HomeRecipes.swift | 242 | Home with no taf: the recipes | |
| MicButton.swift | 122 | The mic in a chat's message bar | |
| NotificationSettingsView.swift | 138 | Account › Alerts | |
| PairSignInView.swift | 104 | Signing in from another phone | KIFF/Pairing |
| PeopleViews.swift | 291 | People and groups | |
| PlanViews.swift | 268 | Buying credits (held for the owner's money question) | |
| QuestionsCard.swift | 121 | Several questions at once | QuestionAnswers |
| RecipeGallery.swift | 135 | The jobs' gallery | |
| RecordSearchViews.swift | 192 | The Record's search | |
| RecordView.swift | 689 | The Record | |
| ReportAnswerSheet.swift | 124 | Report an answer | |
| SetupChatView.swift | 1946 | Coach's conversation | SetupComponent, SetupShape, SentMessages |
| SetupShapePanel.swift | 130 | What Coach has made so far | SetupShape |
| TafChatView.swift | 826 | A taf's chat | |
| TafDetailView.swift | 1467 | A taf's page | Models, NextStep |
| TafDialPanel.swift | 228 | The dial's requests and confirmation | |
| TafKnobView.swift | 377 | The taf's remote | KnobNavigation |
| TafPresentationViews.swift | 492 | A taf's names, mappings and limits, from the service | |
| TafSetupViews.swift | 512 | A taf's Setup | |
| TrialViews.swift | 214 | Trials of a new version | |
| UsageView.swift | 348 | Plan & usage | |
| WorkLog.swift | 181 | What a taf does while it answers | |
| YourStepCard.swift | 46 | A step a taf handed its owner | |

## Not for Linux

| iPad file | Lines | Why |
| :- | -: | :- |
| HomeView.swift | 403 | The phone's Home. A wide screen uses HomeSplitView. |
| TafsBar.swift | 121 | The phone's bar along the foot. A wide screen has the sidebar. |
| PairComputerView.swift | 222 | The phone's side of pairing: the camera reads a computer's code. |
| TafKnobRemote.swift | 64 | Keeps the remote above the phone's tab bar. |

## Shared pieces

| iPad file | Linux |
| :- | :- |
| Design/Theme.swift | `tafs-core/theme.rs`, `style.rs`: every colour, light and dark, the brand sweep and the taf colours |
| Design/Components.swift | `widgets.rs`: `TafAvatar`, `Eyebrow`, `PrimaryLook`, `OutlineLook`, `Panel`, `Legal`, `shortTime`. Not yet: `StateChip`, `Meter`, `DoorRim`, `UnreachableLine` and the rest |
| KIFF/KIFFClient.swift | `tafs-core/signin.rs`, `kiff.rs`: `DeviceStart`, `DeviceToken`, `startDeviceSignIn`, `pollDeviceToken`, `signOut`, `fingerprint`, the refusal's reading, `acceptLanguage`, `screenLanguages` |
| KIFF/PlainError.swift | `signin::plain_words` |
| KIFF/TafsCloud.swift | `Home`, `HomeTaf`, `NeedsYou`, `home()` (`tafs-core/home.rs`, `kiff::home`) |
| KIFF/TafsCloud+Work.swift | `Health`, `TafWork.Summary`, read leniently (`home.rs`) |
| KIFF/KIFFMapping.swift | `taf`, `money`, `window` (`home.rs`); `held`, `says`, `toolWords`, `name`, `reasonBreach`, `moneyReason`, `countReason`, `ordinal`, `details`, `amount`, `tafWords`, `outcomeText` (`held.rs`) |
| KIFF/KIFFClient.swift (held) | `CardException`, `heldCalls`, `answerPage` (`held.rs`, `needs_you.rs`, `kiff::held`) |
| App/AppModel.swift | `startSignIn`, `finishSignIn`, `transient`, `relevantNow`, `needsYouCount`, `accountLabel`, `coachAvailable`, `propose`'s held call (the sample's) |
| Services/ThisDevice.swift | `signin::PLATFORM`, `KIFF_KIND` |
| Services/TafTemplates.swift | `tafs-core/sample.rs`: the five sample tafs and their held calls |
| Model/Models.swift | `Money.figure` for whole euros (`money.rs`); `Window`, `TafCard.hero` (`card.rs`) |

## Coverage

Ported 1, partial 4, not started 41, not for Linux 4: the 50 view files at the pin.
