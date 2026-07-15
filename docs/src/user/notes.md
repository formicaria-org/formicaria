# Creating & organizing notes

## Capture vs. New

- **Capture** — the top box. Type a line and press Enter to create a note
  instantly (its text becomes the body). Pick a **type** (note / task / meeting)
  from the dropdown to label it as you capture.
- **New** — creates a blank note of the chosen type and opens it straight in the
  editor, so you can write a full body and set properties.

## Editing a note

Click any card to open it. It shows the rendered read view. Click **Edit** to
switch to the source editor, where you also get a **properties form**:

| Field   | What it sets              | Format                                   |
|---------|---------------------------|------------------------------------------|
| Type    | `note`/`task`/`meeting`/`asset` | choose from the dropdown           |
| Status  | any label                 | free text (autocompletes known values)   |
| Due     | a deadline                | a date picker (`YYYY-MM-DD`)             |
| Hard    | a hard deadline           | a checkbox                               |
| Title   | an optional title         | free text                                |
| Tags    | tags                      | comma- or space-separated                |

Every change writes back to the note's YAML frontmatter immediately. The body is
plain Markdown, edited as literal text (KaTeX math and Mermaid diagrams render in
the read view). Nothing is ever rewritten behind your back — the file round-trips
byte-for-byte.

## Organizing

There are no folders to manage. You organize by **properties** and see the
results through views: a status board, an agenda of due dates, tag-based boards,
custom properties like `project`. Add any property you like — a new key in a
note's frontmatter flows straight through to grouping with no configuration.
