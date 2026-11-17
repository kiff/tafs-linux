# Home with no taf: the recipes

The iPad's `HomeRecipes` (`HomeRecipes.swift`: `HomeRecipes`, `OwnWordsRow`, `RecipeRow`,
`AppMarks`, `RecipeSheet`) and `TafsCloud.Recipes`, on Linux. The read is
`tafs-core/src/recipes.rs`, tested against the pinned `recipes-home.json`; the screen is
`crates/tafs-app/src/recipes.rs`.

## The read

`GET /v1/recipes` at tafs cloud, with Home, signed in with KIFF (`loadRecipes`). One that can't
be read lists none. It's read exactly as the iPad's decoders read it:

- **Required, or the answer isn't read:** a category's `id` and `title`; a recipe's `id` and
  `title`; `category`, `line` and `say` as words when they're there; `shown` as true or false.
- **Forgiven:** a category's `color` (`#RRGGBB` for light and dark, else none); an app without its
  `name` (left out); `headline`, `today`, `proved` and `sample` (words, else empty; the last two
  trimmed); the Card's lists (a list of words, else none); `connected`, `running`; `home` and
  `labels` (none when they can't be read); `own_words` with no title is none.

## Where it shows

On the iPad, the recipes take the pane beside the sidebar when nothing is picked and
`homeListsRecipes` holds: signed in with KIFF, Home read, no taf, tafs cloud lists them
(`home.shown`), the sheet's words came (`labels`), and there's a recipe in a category. Linux always
has something picked (the Overview first), so there they take the **Overview's** place.

Under Your tafs in the sidebar, a Home with no taf says tafs cloud's line (`homeWords`) where it
sent one; the app's own only when it didn't.

## The pane

At most 720 wide, centred, 24 from the sides:

- tafs cloud's line (17 semibold);
- **Say it in your own words** (`OwnWordsRow`: title 16 semibold, line 14 soft, a chevron), for
  someone who may use Coach. It opens Coach in a new conversation, on no taf (`startNewTaf`), with
  what was in its box kept;
- a chip per category, with two or more: in its colour where tafs cloud sent one (its words and
  edge, and its fill once picked). Picking one narrows the list; picking it again shows every
  recipe;
- each category, in tafs cloud's order, as an eyebrow over its recipes (`RecipeRow`): the problem
  it ends (`headline`, else its title; 16 semibold), its line (14, soft) and its apps by name (12,
  dim), on a 4-wide edge in its category's colour. A category with no recipe isn't shown.

Each app's logo (18) comes before its name where tafs cloud serves one (`AppMark`): only from
tafs cloud's own address at `/v1/logos/{key}`, with a key of 32 lowercase hex digits
(`logoURL`); anything else is no logo. It's read without the session's token, as the iPad's image
load sends none, at most 1 MB, once a launch; while it loads, or when it fails, the name stands
alone. A screen reader hears the name, not the logo.

## The sheet

A recipe opens its sheet (`RecipeSheet`), over the window: the problem it ends (24 bold), its line
(15), what it costs today (14, soft), each zone of its Card in a panel (Does on its own, Asks you
first, Never does: `labels`' words; a zone the Card leaves empty isn't one), its proof run's line
with a check, what that run reported, and **Hand it to Coach** (`labels.hand`) for someone who may
use Coach (`handRecipeToCoach`): the sheet closes, and Coach opens beside the sidebar in a new
conversation with the recipe's words (`say`) in its box, for the person to edit and send. Nothing
is sent. How Coach is opened from elsewhere is `coach_start.rs`; Coach takes it as it opens.

## Screenshots

`--home crates/tafs-app/fixtures/home-empty.json --recipes crates/tafs-app/fixtures/recipes.json`
(both synthetic) draws the pane, CI's `recipes`; with `--page recipe:refunds`, the sheet, as
someone who may use Coach, CI's `recipe-sheet`. `--page coach-hand:refunds` (with `--coach`
crates/tafs-app/fixtures/coach-empty.json) is Coach as Hand it to Coach leaves it, CI's
`coach-hand`.

## The jobs' gallery

Every ready-made job at any time (`RecipeGallerySheet`, `TafsCloud.Gallery`,
`GET /v1/recipes/gallery`, read with Home). Its decoder forgives everything: a category or a job
that can't be read is left out on its own, and words or a strip that can't be read are empty.

In the sidebar, after Your tafs:

- **Explore jobs** (`ExploreRow`; tafs cloud's `words.explore`), with or without tafs and for
  everyone, since browsing creates nothing. With no such words, or no job, there's no row.
- **The strip** (`RecipeStrip`), with tafs, where tafs cloud shows it (`strip.shown`): its title as
  an eyebrow, then its jobs in tafs cloud's order (an id it doesn't list is skipped), each a 4-wide
  bar in its category's colour and its lead (14, two lines); a job opens its sheet. **See all
  jobs** (`words.see_all`, else `words.all`) follows when the strip shows fewer than all of them.

Both open the gallery browsing: its title ("Explore jobs"), "All jobs" over the list, then Home's
list of every job (chips and categories, as above). A job's sheet there is read only: its one
button is "Try it with Coach" (`words.try`), for someone who may use Coach; the label over its
sample is the gallery's (`words.sample`). The sheet's words are the gallery's
`labels`, else `/v1/recipes`'s.

**+ New** opens the gallery to start a taf (`GalleryMode.create`): its title is the gallery's own
(`words.title`), and for someone who may use Coach it leads with **Say it in your own words**
(`words.own_words`, `own_words_line`) with board 8 (a new taf: words in a bubble becoming a card)
beside it, then "All jobs" and every job; a job's sheet hands it with its own words
(`labels.hand`). Either choice closes the gallery first, then Coach opens over Home: the own words
in a new conversation, a job with its words in the box (`actOnGalleryChoice`). A job from the
strip opens the same sheet. Without the gallery, + New opens Coach in a new conversation.

`--home shared/contracts/home.json --gallery crates/tafs-app/fixtures/gallery.json` (synthetic)
draws the Explore row and the strip, CI's `gallery-strip`; with `--page gallery` (and
`--recipes`), the gallery, CI's `gallery`; with `--page new-taf`, as + New opens it, CI's
`new-taf`.

## Not yet

Nothing: every branch of `HomeRecipes` and `RecipeGallery` is drawn and acts.
