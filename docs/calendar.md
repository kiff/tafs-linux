# Calendar

The iPad's Calendar (`CalendarView`, `CalendarModel.swift`), beside the sidebar: the tafs'
scheduled work still to come, in Day, Week and Month. Every occurrence is tafs cloud's
(`GET /v1/calendar?from=…&to=…`), placed by the day and minute it gives in the account's zone:
the app converts no zone. The lines and dates are `tafs-core/src/calendar.rs`, tested there; the
screen is `crates/tafs-app/src/calendar.rs`.

## The screen

- **One read a month:** the selected month's six weeks (from the Monday on or before its 1st), so
  one read draws every view and the navigator's marks. Another month reads again; a view says
  nothing about work until its own range is read. The read is lenient: an event it can't read is
  left out.
- **The header:** Today (a pill as wide as its word; with today shown it does nothing and stays
  readable), previous and next (a day, a week or a month, by the view), the title ("Sun 15
  November 2026", "9 – 15 November 2026", "November 2026", in the language's order), the date
  as a pill that opens a month calendar, and Day / Week / Month. Under it: why the calendar
  couldn't be read and Try again; else "Reading the calendar…"; else tafs cloud's words when the
  account is halted (red) or some work was left out.
- **Wide or compact** by the iPad's rule (`CalendarLayout.of`), on the page's size, drawn again
  when it changes: wide when the page is at least 700 wide, wider than high, and at least 600
  high (a laptop's window); compact otherwise (an upright window).
  - **Wide:** the month navigator (280 wide: its title, previous and next month, its days) beside
    the calendar; Day and Week as time slots, Month as large day cells with how many; the agenda
    under them: the shown dates' work by day, or tafs cloud's words when nothing is to come.
  - **Compact:** Day, the strip of the month's weeks (scrolled to the selected week) and the day's
    time slots; Week, the strip of its seven days and the selected day's work; Month, the day
    cells and the selected day's work.
- **Time slots:** the 24 hours in a view 520 high, scrolled to the first hour with work (8 with
  none), each hour's tafs once with how many times ("×3") in their colour at 16%; in Week a
  heading per day opens that day. "Nothing scheduled on this day." or "…this week." under them.
- **Day cells:** each taf's mark in its colour, at most three; the selected day in the amber at
  18%, today ringed in the amber. The strip's selected day is filled in the amber.
- **A taf's work opens the taf** beside the sidebar; a taf no longer listed opens nothing.

It opens on Week, at today in the account's zone. With no account to read (the sample, or signed
out), the range is empty, as the iPad's screenshot over the sample. `--page calendar` opens Day,
`--page calendar-week` Week; CI also saves Day at an upright iPad's size (`--size 1032x1376`), the
compact layout, as the reference set's `calendar.png`.

## Reading again

The screen reads its month's range when it opens, on Try again, and when pulled past its top
(`.refreshable`: a touchpad's or a touch screen's pull, as GTK reports it) or on Ctrl+R or F5 while
the focus is in it, a computer's way to ask. A pull while a read is on its way asks nothing more.

## Not yet

- **The sample** reads nothing: the iPad's sample, outside a screenshot, tries tafs cloud without
  a session and shows why it couldn't.
