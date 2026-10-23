# Turnitin UI — Original thinking. Lasting impact.

A responsive, independently designed frontend concept inspired by Turnitin’s public product offering. Warm ivory, forest ink, soft sage, and lime create a distinct visual system for the marketing site and educator workspace.

**This is a UI prototype, not an official Turnitin service.** No authentication, plagiarism checking, AI detection, document transmission, or backend integration is implemented. Turnitin and its product names belong to their respective owners.

![Landing page preview](docs/landing-preview.jpg)

## Run locally

Requires Node.js 22.12+ (or 20.19+).

```sh
npm install
npm run dev
```

Open the local URL printed by Vite, normally http://127.0.0.1:5173/.

```sh
npm run build
npm run preview
```

The build performs strict TypeScript checking and produces static assets in `dist/`.

## Screens

| Route               | Experience                                                                                            |
| ------------------- | ----------------------------------------------------------------------------------------------------- |
| `#/`                | Landing page with interactive audience tabs, solution sections, product ecosystem, resources, and FAQ |
| `#/login`           | Clearly labeled demo entry; no credentials requested                                                  |
| `#/app`             | Educator overview, sample activity, review counts, recent submissions                                 |
| `#/app/submissions` | Searchable submissions with review filters and local demo file selection                              |
| `#/app/reports`     | Originality report list and detailed report reader                                                    |
| `#/app/ai`          | Sample AI writing indicators with contextual guidance                                                 |
| `#/app/classes`     | Classes, assignment summaries, and class-specific submission search                                   |
| `#/app/feedback`    | Sample document, editable feedback, and review checklist                                              |
| `#/app/resources`   | Three educational guides, with article dialogs                                                        |
| `#/app/settings`    | Demo profile and preference controls                                                                  |

The report reader has Similarity, AI writing, and Feedback views, illustrative source matches, and a JSON sample export. Dialogs support Escape and keyboard focus containment. The workspace includes mobile navigation, empty search results, validation messages, and notification feedback.

## Demo data and persistence

- All students, documents, classes, scores, source matches, highlights, and activity are illustrative.
- File selection accepts PDF, DOCX, and TXT up to 20 MB. Only filename and size are used. Files are not read, sent, stored, or analyzed.
- Adding a submission generates fixed demo scores; it does not claim to calculate results.
- Submission and feedback state is in React memory and resets on refresh or leaving the workspace. Profile and preference controls keep changes in workspace memory; they are not persisted across refreshes.
- No passwords, API keys, or other credentials are requested or stored.
- The date and assignment schedule are fixed sample content for October 2026.
- Google Fonts loads DM Sans and Manrope; system font fallbacks work offline. Icons use Lucide. All visual compositions are CSS and inline SVG; no copied website assets or screenshot backgrounds are used.

## Structure

```text
src/
  App.tsx                   Hash navigation and page selection
  main.tsx                  React entry point
  styles.css                Shared visual system and responsive styles
  components/
    Brand.tsx               Concept wordmark and shared link
    ReportPreview.tsx       Landing page originality report composition
  pages/
    Landing.tsx             Marketing site
    Login.tsx               Demo entry
    Workspace.tsx           Workspace screens and demo interactions
public/favicon.svg          Custom concept favicon
docs/sample-submission.txt  Safe fixture for checking the local file flow
```

## Future Rust backend integration

The frontend is intentionally independent of the existing Rust backend. Replace the in-memory document data, file-selection handler, and feedback actions in `Workspace.tsx` with API calls once the backend contract is supplied. Add real authentication and authorization at that stage. No backend folder was accessed or changed in this task.

Hash navigation allows the static build to work without server-side route rewrites. To deploy under a subpath such as GitHub Pages, build with `npm run build -- --base=/turnitin-ui/`.

## Content reference

The public [Turnitin homepage](https://www.turnitin.com/) was reviewed in a browser for the product themes: learning integrity, similarity, AI writing insights, feedback, research integrity, grading, and secure assessment. The authenticated workspace is an original conceptual design, not a reproduction of a private Turnitin account. Marketing copy was rewritten for this prototype.
