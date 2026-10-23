import { useEffect, useRef, useState } from "react";
import {
  LayoutDashboard,
  Files,
  GraduationCap,
  Sparkles,
  MessageSquare,
  BookOpen,
  Settings,
  ArrowUpRight,
  ArrowRight,
  Plus,
  Search,
  Bell,
  ChevronDown,
  ChevronRight,
  MoreHorizontal,
  FileText,
  Upload,
  X,
  Check,
  ShieldCheck,
  Fingerprint,
  Download,
  Clock,
  ArrowLeft,
  Menu,
  LogOut,
  CheckCircle2,
} from "lucide-react";
import { Brand } from "../components/Brand";
type DocumentItem = {
  id: number;
  name: string;
  course: string;
  author: string;
  initials: string;
  date: string;
  similarity: number;
  ai: number;
  status: string;
  words: number;
};
const startingDocs: DocumentItem[] = [
  {
    id: 1,
    name: "The changing landscape of urban ecology",
    course: "Environmental Studies",
    author: "Alex Lewis",
    initials: "AL",
    date: "Oct 23, 2026",
    similarity: 8,
    ai: 0,
    status: "Reviewed",
    words: 2450,
  },
  {
    id: 2,
    name: "Ethics in the age of artificial intelligence",
    course: "Philosophy & Ethics",
    author: "Maya Patel",
    initials: "MP",
    date: "Oct 23, 2026",
    similarity: 14,
    ai: 22,
    status: "Needs review",
    words: 3180,
  },
  {
    id: 3,
    name: "Rethinking sustainable economic growth",
    course: "Environmental Studies",
    author: "James Wilson",
    initials: "JW",
    date: "Oct 22, 2026",
    similarity: 5,
    ai: 0,
    status: "Reviewed",
    words: 2800,
  },
  {
    id: 4,
    name: "The language of belonging",
    course: "Academic Writing",
    author: "Sofia Chen",
    initials: "SC",
    date: "Oct 22, 2026",
    similarity: 27,
    ai: 0,
    status: "Needs review",
    words: 1920,
  },
  {
    id: 5,
    name: "Community-led approaches to conservation",
    course: "Environmental Studies",
    author: "Noah Martin",
    initials: "NM",
    date: "Oct 21, 2026",
    similarity: 11,
    ai: 0,
    status: "Reviewed",
    words: 2240,
  },
  {
    id: 6,
    name: "A critical reading of modern narratives",
    course: "Academic Writing",
    author: "Emma Davis",
    initials: "ED",
    date: "Oct 21, 2026",
    similarity: 6,
    ai: 0,
    status: "Reviewed",
    words: 2640,
  },
];
const navItems = [
  { id: "overview", label: "Overview", icon: LayoutDashboard },
  { id: "submissions", label: "Submissions", icon: Files },
  { id: "reports", label: "Originality reports", icon: Fingerprint },
  { id: "ai", label: "AI writing insights", icon: Sparkles },
  { id: "classes", label: "Classes & assignments", icon: GraduationCap },
  { id: "feedback", label: "Feedback studio", icon: MessageSquare },
];
const articles = [
  {
    title: "AI in the classroom: start with the right questions.",
    tag: "RESPONSIBLE AI",
    icon: Sparkles,
    body: "Begin with the learning goal. Discuss when AI assistance is appropriate, what students should disclose, and how their own reasoning should appear in the final work. Ask students to keep drafts and explain the choices they made. A clear, shared policy creates space for honest conversations.",
  },
  {
    title: "A similarity score is the beginning of a conversation.",
    tag: "ACADEMIC INTEGRITY",
    icon: Fingerprint,
    body: "A similarity score measures matching text. It does not determine plagiarism. Quotations, references, and common phrases can appear as matches. Review the source, attribution, and context of each passage. Use the report to help students understand citation and develop their own perspective.",
  },
  {
    title: "Small comments. Meaningful steps forward.",
    tag: "BETTER FEEDBACK",
    icon: MessageSquare,
    body: "Make feedback specific, actionable, and connected to the learning objective. Highlight a strength, identify one useful next step, and invite the student to respond. A question can help a writer think more deeply than a correction alone. Leave space for revision and reflection.",
  },
];
export default function Workspace({ route }: { route: string }) {
  const section =
    route.replace("#/app", "").split("?")[0].split("/")[1] || "overview";
  const [docs, setDocs] = useState(startingDocs);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("All submissions");
  const [upload, setUpload] = useState(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [uploadError, setUploadError] = useState("");
  const [selected, setSelected] = useState<DocumentItem | null>(null);
  const [reportTab, setReportTab] = useState("Similarity");
  const [toast, setToast] = useState("");
  const [notifications, setNotifications] = useState(false);
  const [mobileNav, setMobileNav] = useState(false);
  const [article, setArticle] = useState<number | null>(() =>
    route.includes("?article=") ? Number(route.split("?article=")[1]) : null,
  );
  const [feedback, setFeedback] = useState("");
  const [savedFeedback, setSavedFeedback] = useState(false);
  const [settingsSaved, setSettingsSaved] = useState(false);
  const [preferences, setPreferences] = useState({
    name: "Jordan Morgan",
    role: "Educator",
    institution: "Demo institution",
    language: "English",
    reminders: true,
    resources: true,
  });
  const updatePreference = (
    key: keyof typeof preferences,
    value: string | boolean,
  ) => {
    setPreferences((previous) => ({ ...previous, [key]: value }));
    setSettingsSaved(false);
  };
  const fileInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const query = route.split("?")[1];
    const requested = new URLSearchParams(query || "").get("article");
    if (
      requested !== null &&
      Number(requested) >= 0 &&
      Number(requested) < articles.length
    )
      setArticle(Number(requested));
  }, [route]);
  useEffect(() => {
    const open = upload || selected !== null || article !== null;
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    const oldOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setUpload(false);
        setSelected(null);
        setArticle(null);
      }
      if (e.key === "Tab") {
        const dialog = document.querySelector<HTMLElement>('[role="dialog"]');
        const focusables = dialog?.querySelectorAll<HTMLElement>(
          "a[href],button:not(:disabled),input:not([hidden]),textarea,select",
        );
        if (!focusables?.length) return;
        const first = focusables[0],
          last = focusables[focusables.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", onKey);
    const first = document.querySelector<HTMLElement>('[role="dialog"] button');
    first?.focus();
    return () => {
      document.body.style.overflow = oldOverflow;
      document.removeEventListener("keydown", onKey);
      previous?.focus();
    };
  }, [upload, selected, article]);
  const notify = (message: string) => {
    setToast(message);
    window.setTimeout(() => setToast(""), 4000);
  };
  const visible = docs.filter(
    (d) =>
      `${d.name} ${d.author} ${d.course}`
        .toLowerCase()
        .includes(search.toLowerCase()) &&
      (filter !== "Needs review" || d.status === "Needs review"),
  );
  const needsReview = docs.filter((d) => d.status === "Needs review").length;
  const selectFile = (file: File | undefined) => {
    if (!file) return;
    if (!/\.(pdf|docx|txt)$/i.test(file.name)) {
      setUploadError("Choose a PDF, DOCX, or TXT file.");
      return;
    }
    if (file.size > 20 * 1024 * 1024) {
      setUploadError("Please choose a file smaller than 20 MB.");
      return;
    }
    setSelectedFile(file);
    setUploadError("");
  };
  const addDemoDocument = () => {
    if (!selectedFile) return;
    setDocs((prev) => [
      {
        id: Date.now(),
        name: selectedFile.name.replace(/\.[^.]+$/, ""),
        course: "Academic Writing",
        author: "Jordan Morgan",
        initials: "JM",
        date: "Oct 23, 2026",
        similarity: 8,
        ai: 0,
        status: "Demo report",
        words: 2450,
      },
      ...prev,
    ]);
    setUpload(false);
    setSelectedFile(null);
    notify(
      "Demo submission added. Scores are sample values; no analysis was performed.",
    );
    window.location.hash = "/app/submissions";
  };
  const titles: Record<string, string> = {
    overview: "Your learning, in perspective.",
    submissions: "Every submission. A new perspective.",
    reports: "Originality, in context.",
    ai: "A clearer view of AI writing.",
    classes: "Room for original thinkers.",
    feedback: "Feedback that moves learning forward.",
    resources: "A little knowledge. A fresh perspective.",
    settings: "Make this space your own.",
  };
  const downloadSample = () => {
    const blob = new Blob(
      [
        JSON.stringify(
          {
            prototype: true,
            notice: "Illustrative UI data. Not an actual analysis.",
            document: selected,
          },
          null,
          2,
        ),
      ],
      { type: "application/json" },
    );
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "sample-originality-report.json";
    a.click();
    URL.revokeObjectURL(url);
    notify("Sample report downloaded.");
  };
  return (
    <div className="workspace">
      <aside className={`sidebar ${mobileNav ? "is-open" : ""}`}>
        <Brand />
        <div className="workspace-switch">
          <span className="institution-icon">
            <GraduationCap size={20} />
          </span>
          <div>
            <strong>Learning workspace</strong>
            <span>Educator · Demo institution</span>
          </div>
          <ChevronDown size={13} />
        </div>
        <span className="sidebar-label">YOUR WORKSPACE</span>
        <nav>
          {navItems.map(({ id, label, icon: Icon }) => (
            <a
              key={id}
              className={section === id ? "active" : ""}
              href={`#/app${id === "overview" ? "" : "/" + id}`}
              onClick={() => {
                setSearch("");
                setMobileNav(false);
              }}
            >
              <Icon size={18} />
              {label}
              {id === "submissions" && (
                <span className="nav-count">{docs.length}</span>
              )}
            </a>
          ))}
        </nav>
        <span className="sidebar-label sidebar-second-label">GOOD TO KNOW</span>
        <nav>
          <a
            href="#/app/resources"
            className={section === "resources" ? "active" : ""}
          >
            <BookOpen size={18} />
            Resource library
          </a>
          <a
            href="#/app/settings"
            className={section === "settings" ? "active" : ""}
          >
            <Settings size={18} />
            Settings
          </a>
        </nav>
        <div className="sidebar-bottom">
          <div className="sidebar-promo">
            <span>✳</span>
            <strong>
              Stay curious.
              <br />
              Stay original.
            </strong>
            <p>A little inspiration for your next big idea.</p>
            <a href="#/app/resources">
              Explore resources <ArrowUpRight size={15} />
            </a>
          </div>
          <a className="profile" href="#/app/settings">
            <span className="avatar">JM</span>
            <div>
              <strong>Jordan Morgan</strong>
              <span>Educator</span>
            </div>
            <MoreHorizontal size={17} />
          </a>
          <a className="back-home" href="#/">
            <LogOut size={13} /> Back to website
          </a>
        </div>
      </aside>
      {mobileNav && (
        <button
          className="sidebar-overlay"
          aria-label="Close navigation"
          onClick={() => setMobileNav(false)}
        />
      )}
      <div className="workspace-main">
        <header className="workspace-topbar">
          <div className="breadcrumbs">
            <button
              className="app-menu"
              aria-label="Open navigation"
              onClick={() => setMobileNav(true)}
            >
              <Menu size={20} />
            </button>
            <span>Workspace</span>
            <ChevronRight size={13} />
            <b>
              {navItems.find((n) => n.id === section)?.label ||
                section.charAt(0).toUpperCase() + section.slice(1)}
            </b>
          </div>
          <div className="topbar-right">
            <span className="demo-pill">
              <span className="live-dot" /> Demo workspace
            </span>
            <span className="topbar-divider" />
            <div className="notification-wrap">
              <button
                className="notification-button"
                aria-label="Notifications"
                aria-expanded={notifications}
                onClick={() => setNotifications(!notifications)}
              >
                <Bell size={19} />
                <i />
              </button>
              {notifications && (
                <div className="notification-panel">
                  <strong>You’re all caught up.</strong>
                  <p>Sample workspace notifications will appear here.</p>
                </div>
              )}
            </div>
            <a
              href="#/app/settings"
              className="avatar small-avatar"
              aria-label="Profile settings"
            >
              JM
            </a>
          </div>
        </header>
        <main className="app-content">
          <div className="app-heading">
            <div>
              <div className="eyebrow">
                {section === "overview"
                  ? "FRIDAY, OCTOBER 23, 2026"
                  : "MADE FOR MEANINGFUL LEARNING"}
              </div>
              <h1>{titles[section] || titles.overview}</h1>
              <p>
                {section === "overview"
                  ? "Welcome back, Jordan. Here’s where things stand today."
                  : "Explore a more thoughtful approach to academic integrity."}
              </p>
            </div>
            {!["resources", "settings", "feedback"].includes(section) && (
              <button
                className="button button-dark"
                onClick={() => setUpload(true)}
              >
                <Plus size={17} />
                New submission
              </button>
            )}
          </div>
          {section === "overview" && (
            <>
              <div className="welcome-banner">
                <div>
                  <span className="banner-tag">A LITTLE PERSPECTIVE</span>
                  <h2>
                    Behind every submission,
                    <br />
                    there’s an original thinker.
                  </h2>
                  <p>Give good ideas the attention they deserve.</p>
                  <a href="#/app/reports">
                    Take a closer look <ArrowRight size={15} />
                  </a>
                </div>
                <div className="banner-art">
                  <div className="art-sheet">
                    <Fingerprint size={55} strokeWidth={1} />
                    <span>
                      Make your
                      <br />
                      <i>mark.</i>
                    </span>
                  </div>
                  <span className="banner-star">✳</span>
                </div>
              </div>
              <div className="stats-grid">
                {[
                  {
                    icon: Files,
                    label: "Total submissions",
                    value: String(docs.length).padStart(2, "0"),
                    note: "Across your active classes",
                    color: "sage",
                  },
                  {
                    icon: CheckCircle2,
                    label: "Reviewed submissions",
                    value: String(
                      docs.filter((d) => d.status === "Reviewed").length,
                    ).padStart(2, "0"),
                    note: "Ready for the next step",
                    color: "lavender",
                  },
                  {
                    icon: Clock,
                    label: "Waiting for review",
                    value: String(needsReview).padStart(2, "0"),
                    note: "A little attention goes a long way",
                    color: "peach",
                  },
                  {
                    icon: GraduationCap,
                    label: "Active classes",
                    value: "03",
                    note: "A world of different perspectives",
                    color: "yellow",
                  },
                ].map(({ icon: Icon, label, value, note, color }) => (
                  <div className="stat-card" key={label}>
                    <div>
                      <span>{label}</span>
                      <span className={`stat-icon ${color}`}>
                        <Icon size={18} />
                      </span>
                    </div>
                    <strong>{value}</strong>
                    <p>{note}</p>
                  </div>
                ))}
              </div>
            </>
          )}
          {["overview", "submissions", "reports", "ai"].includes(section) && (
            <>
              {section === "ai" && (
                <div className="info-banner">
                  <Sparkles size={22} />
                  <div>
                    <strong>
                      Insights to inform a conversation, not determine an
                      outcome.
                    </strong>
                    <p>
                      AI writing indicators are not definitive evidence. Review
                      them alongside the writing process, context, and your
                      institution’s policy. All values here are illustrative.
                    </p>
                  </div>
                </div>
              )}
              <section className="submissions-panel">
                <div className="panel-heading">
                  <div>
                    <h2>
                      {section === "overview"
                        ? "Recent submissions"
                        : section === "ai"
                          ? "Writing insights"
                          : "Your submissions"}
                    </h2>
                    <span>
                      {section === "overview"
                        ? "A fresh look at the work coming in."
                        : "Every report starts with a closer look."}
                    </span>
                  </div>
                  {section === "overview" ? (
                    <a href="#/app/submissions">
                      View all submissions <ArrowUpRight size={15} />
                    </a>
                  ) : (
                    <span className="sample-label">ILLUSTRATIVE DATA</span>
                  )}
                </div>
                <div className="table-toolbar">
                  <div className="table-tabs">
                    <button
                      className={filter === "All submissions" ? "active" : ""}
                      onClick={() => setFilter("All submissions")}
                    >
                      All submissions <span>{docs.length}</span>
                    </button>
                    <button
                      className={filter === "Needs review" ? "active" : ""}
                      onClick={() => setFilter("Needs review")}
                    >
                      Needs review <span>{needsReview}</span>
                    </button>
                  </div>
                  <label className="table-search">
                    <Search size={16} />
                    <input
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                      placeholder="Search submissions…"
                      aria-label="Search submissions"
                    />
                  </label>
                </div>
                <div className="table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th>DOCUMENT</th>
                        <th>STUDENT</th>
                        <th>
                          {section === "ai" ? "AI INDICATOR" : "SIMILARITY"}
                        </th>
                        <th>STATUS</th>
                        <th>SUBMITTED</th>
                        <th>
                          <span className="sr-only">Open report</span>
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {visible
                        .slice(0, section === "overview" ? 5 : undefined)
                        .map((d) => (
                          <tr
                            key={d.id}
                            onClick={() => {
                              setSelected(d);
                              setReportTab(
                                section === "ai" ? "AI writing" : "Similarity",
                              );
                            }}
                          >
                            <td>
                              <button className="document-cell">
                                <span className="document-icon">
                                  <FileText size={19} />
                                </span>
                                <span>
                                  <strong>{d.name}</strong>
                                  <small>{d.course}</small>
                                </span>
                              </button>
                            </td>
                            <td>
                              <span className="student-cell">
                                <span
                                  className={`tiny-avatar avatar-${d.id % 3}`}
                                >
                                  {d.initials}
                                </span>
                                {d.author}
                              </span>
                            </td>
                            <td>
                              <span
                                className={`similarity-pill ${(section === "ai" ? d.ai : d.similarity) >= 20 ? "medium" : "low"}`}
                              >
                                <i />
                                {section === "ai" ? d.ai : d.similarity}%
                              </span>
                            </td>
                            <td>
                              <span
                                className={`status ${d.status === "Needs review" ? "needs-review" : ""}`}
                              >
                                <i />
                                {d.status}
                              </span>
                            </td>
                            <td className="date-cell">{d.date}</td>
                            <td>
                              <button
                                className="table-open"
                                aria-label={`Open report for ${d.name}`}
                              >
                                <ArrowUpRight size={17} />
                              </button>
                            </td>
                          </tr>
                        ))}
                    </tbody>
                  </table>
                  {visible.length === 0 && (
                    <div className="empty-state">
                      <Search size={28} />
                      <h3>No submissions found</h3>
                      <p>Try a different title, student, or class.</p>
                      <button
                        className="text-button"
                        onClick={() => {
                          setSearch("");
                          setFilter("All submissions");
                        }}
                      >
                        Clear search <ArrowRight size={15} />
                      </button>
                    </div>
                  )}
                </div>
                <div className="table-footer">
                  <span>
                    Showing{" "}
                    {Math.min(
                      visible.length,
                      section === "overview" ? 5 : visible.length,
                    )}{" "}
                    of {visible.length} submissions
                  </span>
                  <span>
                    <ShieldCheck size={13} /> Demo data · Human judgment, always
                  </span>
                </div>
              </section>
              {section === "overview" && (
                <div className="app-bottom-grid">
                  <div className="review-tip">
                    <span className="stat-icon lavender">
                      <Sparkles size={19} />
                    </span>
                    <div>
                      <h3>A score is just the beginning.</h3>
                      <p>
                        Context makes all the difference. Open a report to
                        explore sources and start a meaningful conversation.
                      </p>
                      <a href="#/app/resources?article=1">
                        A guide to similarity <ArrowUpRight size={15} />
                      </a>
                    </div>
                  </div>
                  <div className="activity-card">
                    <h3>Coming up in your classes</h3>
                    <div>
                      <span className="calendar-tile">
                        OCT<b>26</b>
                      </span>
                      <div>
                        <strong>Research reflection · Final draft</strong>
                        <span>Academic Writing · 24 students</span>
                      </div>
                      <a
                        href="#/app/classes"
                        aria-label="View upcoming assignment"
                      >
                        <ArrowUpRight size={17} />
                      </a>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
          {section === "classes" && (
            <div className="classes-grid">
              {[
                {
                  name: "Environmental Studies",
                  code: "ENV 201",
                  students: 32,
                  assignments: 4,
                  color: "sage",
                  description: "New perspectives on our changing world.",
                },
                {
                  name: "Philosophy & Ethics",
                  code: "PHIL 104",
                  students: 28,
                  assignments: 3,
                  color: "lavender",
                  description: "Good questions. Thoughtful conversations.",
                },
                {
                  name: "Academic Writing",
                  code: "ENG 102",
                  students: 24,
                  assignments: 5,
                  color: "peach",
                  description: "Finding a voice that’s entirely your own.",
                },
              ].map((c, i) => (
                <article className="class-card" key={c.code}>
                  <div className={`class-art ${c.color}`}>
                    <span>{c.code}</span>
                    <span className="class-art-icon">
                      {i === 0 ? "✳" : i === 1 ? "◈" : "Aa"}
                    </span>
                  </div>
                  <div className="class-details">
                    <span className="small-label">FALL SEMESTER · 2026</span>
                    <h2>{c.name}</h2>
                    <p>{c.description}</p>
                    <div className="class-meta">
                      <span>{c.students} students</span>
                      <span>{c.assignments} assignments</span>
                    </div>
                    <button
                      className="arrow-link"
                      onClick={() => {
                        setFilter("All submissions");
                        setSearch(c.name);
                        window.location.hash = "/app/submissions";
                      }}
                    >
                      View submissions <ArrowUpRight size={16} />
                    </button>
                  </div>
                </article>
              ))}
              <div className="assignment-panel">
                <h2>Upcoming assignments</h2>
                <div>
                  <span className="calendar-tile">
                    OCT<b>26</b>
                  </span>
                  <div>
                    <strong>Research reflection · Final draft</strong>
                    <p>Academic Writing · Due 11:59 PM</p>
                  </div>
                  <button
                    className="button button-outline small"
                    onClick={() => {
                      setUpload(true);
                    }}
                  >
                    Add submission <Plus size={15} />
                  </button>
                </div>
                <div>
                  <span className="calendar-tile">
                    OCT<b>29</b>
                  </span>
                  <div>
                    <strong>Ethics case study · First draft</strong>
                    <p>Philosophy & Ethics · Due 11:59 PM</p>
                  </div>
                  <button
                    className="button button-outline small"
                    onClick={() => setUpload(true)}
                  >
                    Add submission <Plus size={15} />
                  </button>
                </div>
              </div>
            </div>
          )}
          {section === "feedback" && (
            <div className="feedback-layout">
              <section className="feedback-document">
                <div className="panel-heading">
                  <div>
                    <h2>A perspective of my own</h2>
                    <span>Alex Lewis · Environmental Studies</span>
                  </div>
                  <span className="sample-label">SAMPLE DRAFT</span>
                </div>
                <article className="essay">
                  <span>ENVIRONMENTAL STUDIES · REFLECTIVE ESSAY</span>
                  <h2>
                    The changing landscape
                    <br />
                    of urban ecology
                  </h2>
                  <p className="essay-author">Alex Lewis · October 23, 2026</p>
                  <h3>A different kind of ecosystem</h3>
                  <p>
                    Cities are often described as places apart from nature. Yet
                    the trees along a busy street, the birds nesting under a
                    bridge, and the gardens between buildings tell a different
                    story.
                  </p>
                  <p>
                    <mark>
                      Urban ecology invites us to see the city as a living
                      system.
                    </mark>{" "}
                    Each small patch of green creates connections between
                    people, places, and the wider environment.
                  </p>
                  <h3>Making room for change</h3>
                  <p>
                    When communities take part in shaping their surroundings,
                    the benefits reach beyond biodiversity. Shared spaces become
                    places for learning, conversation, and a growing sense of
                    belonging.
                  </p>
                  <p>
                    The question is not simply how to add more green to a city.
                    It is how to build a relationship with the life that is
                    already there.
                  </p>
                </article>
              </section>
              <aside className="feedback-sidebar">
                <span className="stat-icon peach">
                  <MessageSquare size={22} />
                </span>
                <h2>
                  A little feedback.
                  <br />A big step forward.
                </h2>
                <p>Help Alex see what’s working and where to go next.</p>
                <label htmlFor="feedback">Your feedback</label>
                <textarea
                  id="feedback"
                  value={feedback}
                  onChange={(e) => {
                    setFeedback(e.target.value);
                    setSavedFeedback(false);
                  }}
                  placeholder="Your perspective on cities as living systems is compelling. Could you explore a specific example?"
                />
                <button
                  className="button button-dark"
                  disabled={!feedback.trim()}
                  onClick={() => {
                    setSavedFeedback(true);
                    notify("Feedback saved for this demo session.");
                  }}
                >
                  {savedFeedback ? (
                    <Check size={16} />
                  ) : (
                    <MessageSquare size={16} />
                  )}{" "}
                  {savedFeedback ? "Feedback saved" : "Save demo feedback"}
                </button>
                <div className="feedback-rubric">
                  <h3>Review checklist</h3>
                  {[
                    "Clear central idea",
                    "Evidence & attribution",
                    "Structure & flow",
                    "Original perspective",
                  ].map((t) => (
                    <label key={t}>
                      <input type="checkbox" />
                      {t}
                    </label>
                  ))}
                </div>
                <small>Comments are stored only while this page is open.</small>
              </aside>
            </div>
          )}
          {section === "resources" && (
            <>
              <div className="resource-intro">
                <BookOpen size={30} />
                <div>
                  <h2>Good ideas start with understanding.</h2>
                  <p>
                    Short guides to thoughtful assessment and responsible AI.
                    Written for this prototype.
                  </p>
                </div>
              </div>
              <div className="app-resources-grid">
                {articles.map((a, i) => (
                  <button
                    className="app-resource"
                    key={a.title}
                    onClick={() => setArticle(i)}
                  >
                    <div
                      className={`resource-art ${i === 0 ? "resource-ai" : i === 1 ? "resource-similarity" : "resource-feedback"}`}
                    >
                      <a.icon size={60} strokeWidth={1} />
                      <ArrowUpRight size={21} />
                    </div>
                    <div>
                      <span className="small-label">{a.tag} · 1 MIN READ</span>
                      <h2>{a.title}</h2>
                      <span className="arrow-link">
                        Read the guide <ArrowRight size={16} />
                      </span>
                    </div>
                  </button>
                ))}
              </div>
            </>
          )}
          {section === "settings" && (
            <form
              className="settings-panel"
              onSubmit={(e) => {
                e.preventDefault();
                setSettingsSaved(true);
                notify("Preferences saved for this demo session.");
              }}
            >
              <div className="panel-heading">
                <div>
                  <h2>Your profile</h2>
                  <span>A few things that make this workspace yours.</span>
                </div>
                <span className="avatar">JM</span>
              </div>
              <div className="settings-fields">
                <label>
                  Full name
                  <input
                    value={preferences.name}
                    onChange={(e) => updatePreference("name", e.target.value)}
                    required
                  />
                </label>
                <label>
                  Role
                  <select
                    value={preferences.role}
                    onChange={(e) => updatePreference("role", e.target.value)}
                  >
                    <option>Educator</option>
                    <option>Student</option>
                    <option>Researcher</option>
                  </select>
                </label>
                <label>
                  Institution
                  <input
                    value={preferences.institution}
                    onChange={(e) =>
                      updatePreference("institution", e.target.value)
                    }
                  />
                </label>
                <label>
                  Language
                  <select
                    value={preferences.language}
                    onChange={(e) =>
                      updatePreference("language", e.target.value)
                    }
                  >
                    <option>English</option>
                    <option>हिन्दी</option>
                    <option>Español</option>
                  </select>
                </label>
              </div>
              <div className="settings-toggle">
                <div>
                  <strong>Review reminders</strong>
                  <p>Show reminders for sample submissions awaiting review.</p>
                </div>
                <label className="toggle">
                  <input
                    type="checkbox"
                    aria-label="Review reminders"
                    checked={preferences.reminders}
                    onChange={(e) =>
                      updatePreference("reminders", e.target.checked)
                    }
                  />
                  <span />
                </label>
              </div>
              <div className="settings-toggle">
                <div>
                  <strong>Learning resources</strong>
                  <p>Include helpful guides in your demo workspace.</p>
                </div>
                <label className="toggle">
                  <input
                    type="checkbox"
                    aria-label="Learning resources"
                    checked={preferences.resources}
                    onChange={(e) =>
                      updatePreference("resources", e.target.checked)
                    }
                  />
                  <span />
                </label>
              </div>
              <div className="settings-bottom">
                <span>Preferences are held in this demo session only.</span>
                <button className="button button-dark" type="submit">
                  {settingsSaved ? "Preferences saved" : "Save preferences"}
                  <Check size={16} />
                </button>
              </div>
            </form>
          )}
          <footer className="workspace-footer">
            <span>Room for original thinking.</span>
            <span>
              Independent UI concept · No real document analysis <span>✳</span>
            </span>
          </footer>
        </main>
      </div>
      {upload && (
        <div className="modal-backdrop" onClick={() => setUpload(false)}>
          <section
            className="upload-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="upload-title"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              className="modal-close"
              aria-label="Close upload"
              onClick={() => setUpload(false)}
            >
              <X size={20} />
            </button>
            <span className="modal-icon">
              <Upload size={25} />
            </span>
            <div className="eyebrow">EVERY IDEA STARTS SOMEWHERE.</div>
            <h2 id="upload-title">Make room for your next idea.</h2>
            <p>Add a document to explore the submission experience.</p>
            <button
              className={`dropzone ${selectedFile ? "has-file" : ""}`}
              onClick={() => fileInput.current?.click()}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                selectFile(e.dataTransfer.files[0]);
              }}
            >
              {selectedFile ? <FileText size={32} /> : <Upload size={32} />}
              <strong>
                {selectedFile ? selectedFile.name : "Drop your document here"}
              </strong>
              <span>
                {selectedFile
                  ? `${(selectedFile.size / 1024).toFixed(1)} KB · Ready for demo`
                  : "or click to browse your files"}
              </span>
              <small>PDF, DOCX, or TXT · Up to 20 MB</small>
            </button>
            <input
              type="file"
              accept=".pdf,.docx,.txt"
              ref={fileInput}
              hidden
              onChange={(e) => selectFile(e.target.files?.[0])}
            />
            {uploadError && (
              <p className="form-error" role="alert">
                {uploadError}
              </p>
            )}
            <div className="upload-disclaimer">
              <ShieldCheck size={18} />
              <p>
                <b>This is a local UI demo.</b> Your file is not uploaded, read,
                or analyzed. The added report uses fixed sample scores.
              </p>
            </div>
            <button
              className="button button-dark"
              disabled={!selectedFile}
              onClick={addDemoDocument}
            >
              Add demo submission <ArrowUpRight size={17} />
            </button>
          </section>
        </div>
      )}
      {selected && (
        <div className="report-backdrop">
          <section
            className="report-viewer"
            role="dialog"
            aria-modal="true"
            aria-labelledby="report-title"
          >
            <header className="report-viewer-header">
              <button className="back-link" onClick={() => setSelected(null)}>
                <ArrowLeft size={17} />
                Back to workspace
              </button>
              <span className="demo-pill">Illustrative report</span>
              <button
                className="button button-outline small"
                onClick={downloadSample}
              >
                <Download size={15} />
                Sample export
              </button>
              <button
                className="icon-button"
                aria-label="Close report"
                onClick={() => setSelected(null)}
              >
                <X size={20} />
              </button>
            </header>
            <div className="report-viewer-body">
              <div className="report-paper-wrap">
                <div className="report-document-header">
                  <span className="tiny-avatar">{selected.initials}</span>
                  <div>
                    <strong>{selected.author}</strong>
                    <span>
                      {selected.course} · {selected.words.toLocaleString()}{" "}
                      words
                    </span>
                  </div>
                  <span>{selected.date}</span>
                </div>
                <article className="essay report-paper">
                  <span>ORIGINALITY REPORT · SAMPLE CONTENT</span>
                  <h2 id="report-title">{selected.name}</h2>
                  <p className="essay-author">
                    {selected.author} · {selected.date}
                  </p>
                  <h3>Introduction</h3>
                  <p>
                    Original thinking begins with a willingness to look at
                    familiar questions from a different perspective. By
                    exploring the relationship between established ideas and new
                    observations, we create room for deeper understanding.
                  </p>
                  <p>
                    <mark
                      className={reportTab === "AI writing" ? "ai-mark" : ""}
                    >
                      The connections between people and their environments
                      shape how communities learn, adapt, and grow.
                    </mark>{" "}
                    This paper considers the ways in which thoughtful inquiry
                    can bring those connections into focus.
                  </p>
                  <h3>A broader perspective</h3>
                  <p>
                    When we consider a single issue in isolation, it is easy to
                    overlook the systems that surround it. An approach grounded
                    in context invites us to examine not only what we know, but
                    how that knowledge was developed.
                  </p>
                  <p>
                    <mark>
                      Meaningful change depends on collaboration, careful
                      observation, and an openness to revise our assumptions.
                    </mark>{" "}
                    These qualities support work that is both rigorous and
                    responsive to the world around it.
                  </p>
                  <h3>Looking ahead</h3>
                  <p>
                    The most valuable outcome may be a better question. By
                    remaining curious and attentive to the evidence, we can
                    continue to build a perspective that is informed,
                    reflective, and our own.
                  </p>
                  <div className="essay-end">END OF SAMPLE DOCUMENT</div>
                </article>
              </div>
              <aside className="report-insights">
                <div className="report-insight-tabs">
                  {["Similarity", "AI writing", "Feedback"].map((t) => (
                    <button
                      className={reportTab === t ? "active" : ""}
                      onClick={() => setReportTab(t)}
                      key={t}
                    >
                      {t}
                    </button>
                  ))}
                </div>
                {reportTab === "Similarity" ? (
                  <>
                    <span className="eyebrow">A CLOSER LOOK</span>
                    <h2>Every match has context.</h2>
                    <div className="big-score">
                      <strong>
                        {selected.similarity}
                        <span>%</span>
                      </strong>
                      <span>
                        Overall similarity
                        <br />
                        <small>Sample score</small>
                      </span>
                    </div>
                    <div className="score-progress">
                      <span style={{ width: `${selected.similarity}%` }} />
                    </div>
                    <p className="insight-note">
                      Matched text is a starting point for review. A similarity
                      score does not determine plagiarism.
                    </p>
                    <h3>
                      Source overview <span>3 matches</span>
                    </h3>
                    {[
                      {
                        title: "Urban ecosystems and shared spaces",
                        source: "Sample academic journal",
                        percent: Math.max(
                          1,
                          Math.floor(selected.similarity * 0.5),
                        ),
                        color: "yellow",
                      },
                      {
                        title: "Perspectives on community learning",
                        source: "Sample publication",
                        percent: Math.max(
                          1,
                          Math.floor(selected.similarity * 0.3),
                        ),
                        color: "lavender",
                      },
                      {
                        title: "A framework for collaborative change",
                        source: "Sample student paper",
                        percent: Math.max(
                          1,
                          selected.similarity -
                            Math.floor(selected.similarity * 0.5) -
                            Math.floor(selected.similarity * 0.3),
                        ),
                        color: "sage",
                      },
                    ].map((s, i) => (
                      <div className="source-item" key={s.title}>
                        <span className={`source-number ${s.color}`}>
                          {i + 1}
                        </span>
                        <div>
                          <strong>{s.title}</strong>
                          <span>{s.source}</span>
                        </div>
                        <b>{s.percent}%</b>
                      </div>
                    ))}
                    <div className="info-banner small-info">
                      <ShieldCheck size={19} />
                      <p>
                        Source names and highlights are illustrative. No source
                        matching has been performed.
                      </p>
                    </div>
                  </>
                ) : reportTab === "AI writing" ? (
                  <>
                    <span className="stat-icon lavender">
                      <Sparkles size={22} />
                    </span>
                    <h2>Insight. With perspective.</h2>
                    <div className="big-score">
                      <strong>
                        {selected.ai}
                        <span>%</span>
                      </strong>
                      <span>
                        AI writing indicator
                        <br />
                        <small>Illustrative value</small>
                      </span>
                    </div>
                    <p className="insight-note">
                      This sample indicator represents text that might warrant a
                      closer look. It is not a definitive assessment of
                      authorship.
                    </p>
                    <h3>A thoughtful next step</h3>
                    <ul className="insight-list">
                      <li>
                        <Check size={15} /> Review drafts and the writing
                        process
                      </li>
                      <li>
                        <Check size={15} /> Invite the student to share their
                        approach
                      </li>
                      <li>
                        <Check size={15} /> Apply your institution’s AI policy
                      </li>
                    </ul>
                    <div className="info-banner small-info">
                      <ShieldCheck size={20} />
                      <p>
                        Never use an AI indicator alone to make a decision about
                        a student.
                      </p>
                    </div>
                  </>
                ) : (
                  <>
                    <span className="stat-icon peach">
                      <MessageSquare size={21} />
                    </span>
                    <h2>Start a conversation.</h2>
                    <p className="insight-note">
                      Connect feedback with a useful next step.
                    </p>
                    <label className="feedback-label" htmlFor="report-feedback">
                      Your comment
                    </label>
                    <textarea
                      id="report-feedback"
                      value={feedback}
                      onChange={(e) => setFeedback(e.target.value)}
                      placeholder="What’s working? What could the writer explore next?"
                    />
                    <button
                      className="button button-dark"
                      disabled={!feedback.trim()}
                      onClick={() =>
                        notify("Comment saved for this demo session.")
                      }
                    >
                      <Check size={15} />
                      Save demo comment
                    </button>
                  </>
                )}
              </aside>
            </div>
          </section>
        </div>
      )}
      {article !== null && articles[article] && (
        <div className="modal-backdrop" onClick={() => setArticle(null)}>
          <article
            className="article-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="article-title"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              className="modal-close"
              aria-label="Close article"
              onClick={() => setArticle(null)}
            >
              <X size={20} />
            </button>
            <span className="eyebrow">{articles[article].tag}</span>
            <h2 id="article-title">{articles[article].title}</h2>
            {articles[article].body
              .split(". ")
              .reduce<string[][]>((groups, s, i) => {
                if (i % 2 === 0) groups.push([]);
                groups[groups.length - 1].push(s);
                return groups;
              }, [])
              .map((g, i) => (
                <p key={i}>
                  {g.join(". ")}
                  {g.join(". ").endsWith(".") ? "" : "."}
                </p>
              ))}
            <div className="article-footer">
              <BookOpen size={16} />
              Educational overview written for this independent prototype.
            </div>
          </article>
        </div>
      )}
      {toast && (
        <div className="toast" role="status">
          <CheckCircle2 size={19} />
          {toast}
          <button
            aria-label="Dismiss notification"
            onClick={() => setToast("")}
          >
            <X size={15} />
          </button>
        </div>
      )}
    </div>
  );
}
