import { Link } from "../components/Navigation";
import { useState } from "react";
import {
  ArrowRight,
  ArrowUpRight,
  Check,
  ChevronDown,
  Fingerprint,
  Sparkles,
  MessageSquare,
  ShieldCheck,
  BookOpen,
  Globe2,
  GraduationCap,
  ScanLine,
  X,
  Menu,
  Plus,
  Minus,
} from "lucide-react";
import { Brand, ArrowLink } from "../components/Brand";
import { ReportPreview } from "../components/ReportPreview";
const features = [
  {
    icon: Fingerprint,
    number: "01",
    title: "Make originality\nthe standard.",
    copy: "Look beyond a percentage. Compare writing with a vast collection of academic sources and give every original idea the recognition it deserves.",
    tag: "Similarity checking",
    color: "sage",
  },
  {
    icon: Sparkles,
    number: "02",
    title: "Bring clarity\nto the AI era.",
    copy: "Understand potential AI-generated writing, encourage responsible use, and start informed conversations with students.",
    tag: "AI writing insights",
    color: "lavender",
  },
  {
    icon: MessageSquare,
    number: "03",
    title: "Turn feedback\ninto a way forward.",
    copy: "Make every assignment a learning opportunity with meaningful comments, clear rubrics, and feedback that helps students grow.",
    tag: "Feedback & assessment",
    color: "peach",
  },
];
const questions = [
  [
    "What does Turnitin check?",
    "Turnitin helps educators review text similarity, identify potential AI-generated writing, and provide feedback. Its wider suite also supports research integrity, grading, and secure assessments.",
  ],
  [
    "Does a similarity score mean plagiarism?",
    "No. Similarity highlights text that matches other sources, including quotations and references. Educators need to review the matches and context before making a judgment.",
  ],
  [
    "How should AI writing insights be used?",
    "AI writing indicators support review and conversation. They are not definitive evidence and should be considered alongside the writing process, institutional policy, and human judgment.",
  ],
  [
    "Can I explore the workspace?",
    "Yes. Choose “Explore the workspace” to open this independent interface concept. Sign in or create an account to explore the workspace. Documents, scores, and insights remain illustrative demo data.",
  ],
];
export default function Landing() {
  const [announcement, setAnnouncement] = useState(true);
  const [menu, setMenu] = useState(false);
  const [faq, setFaq] = useState<number | null>(0);
  const [audience, setAudience] = useState("Higher education");
  return (
    <div className="landing">
      {announcement && (
        <div className="announcement">
          <span className="announcement-pill">A NEW CHAPTER</span>
          <span>Human potential. Meet responsible AI.</span>
          <Link to="#solutions">
            Discover what’s possible <ArrowUpRight size={13} />
          </Link>
          <button
            aria-label="Dismiss announcement"
            onClick={() => setAnnouncement(false)}
          >
            <X size={15} />
          </button>
        </div>
      )}
      <header className="site-header container">
        <Brand />
        <nav className={menu ? "site-nav open" : "site-nav"}>
          <Link to="#solutions" onClick={() => setMenu(false)}>
            Solutions <ChevronDown size={12} />
          </Link>
          <Link to="#approach" onClick={() => setMenu(false)}>
            Our approach
          </Link>
          <Link to="#resources" onClick={() => setMenu(false)}>
            Resources
          </Link>
        </nav>
        <div className="header-actions">
          <Link className="login-link" to="/auth/login">
            Log in <ArrowUpRight size={15} />
          </Link>
          <Link className="button button-dark small" to="/dashboard">
            Explore workspace <ArrowUpRight size={15} />
          </Link>
        </div>
        <button
          className="mobile-menu"
          aria-label="Toggle navigation"
          aria-expanded={menu}
          onClick={() => setMenu(!menu)}
        >
          {menu ? <X /> : <Menu />}
        </button>
      </header>
      <main>
        <section className="hero container">
          <div className="hero-copy">
            <div className="eyebrow">
              <span className="little-star">✳</span> BUILT FOR ORIGINAL THINKING
            </div>
            <h1>
              Big ideas.
              <br />
              Real learning.
              <br />
              <span>All human.</span>
              <span className="heading-asterisk">✳</span>
            </h1>
            <p>
              In a world of AI, original thinking matters more than ever. Give
              educators the clarity to guide it. And students the confidence to
              own it.
            </p>
            <div className="hero-buttons">
              <Link className="button button-dark" to="/dashboard">
                Explore the workspace <ArrowUpRight size={18} />
              </Link>
              <Link className="text-button" to="#solutions">
                Find your solution <ArrowRight size={17} />
              </Link>
            </div>
            <div className="hero-note">
              <span className="mini-avatars">
                <i>J</i>
                <i>M</i>
                <i>A</i>
                <i>S</i>
              </span>
              <span>
                For educators. For students.
                <br />
                <strong>For everything learning can be.</strong>
              </span>
            </div>
          </div>
          <ReportPreview />
        </section>
        <section className="trust-strip container">
          <span>INTEGRITY CONNECTS US.</span>
          <div>
            <GraduationCap />
            <span>Higher education</span>
          </div>
          <div>
            <BookOpen />
            <span>Secondary education</span>
          </div>
          <div>
            <Globe2 />
            <span>Research & publishing</span>
          </div>
          <p>
            One shared purpose.
            <br />
            <b>Learning that lasts.</b>
          </p>
        </section>
        <section
          className="solutions-section section-space container"
          id="solutions"
        >
          <div className="section-intro">
            <div>
              <div className="eyebrow">
                A CLEARER PICTURE. A BETTER WAY FORWARD.
              </div>
              <h2>
                Less uncertainty.
                <br />
                <span>More possibility.</span>
              </h2>
            </div>
            <p>
              From the first draft to the final submission, bring integrity and
              insight to every step of the learning journey.
            </p>
          </div>
          <div className="feature-grid">
            {features.map(({ icon: Icon, number, title, copy, tag, color }) => (
              <article className={`feature-card ${color}`} key={number}>
                <div className="feature-card-top">
                  <span className="feature-icon">
                    <Icon size={25} strokeWidth={1.5} />
                  </span>
                  <span>{number} /</span>
                </div>
                <h3>
                  {title.split("\n").map((t, i) => (
                    <span key={i}>
                      {t}
                      <br />
                    </span>
                  ))}
                </h3>
                <p>{copy}</p>
                <Link
                  to={
                    number === "02"
                      ? "/dashboard/ai"
                      : number === "03"
                        ? "/dashboard/feedback"
                        : "/dashboard/reports"
                  }
                >
                  <span>{tag}</span>
                  <span className="round-arrow">
                    <ArrowUpRight size={18} />
                  </span>
                </Link>
              </article>
            ))}
          </div>
          <div className="solution-footnote">
            <ShieldCheck size={16} />
            <span>
              Thoughtful technology. Informed educators. Human judgment, always.
            </span>
          </div>
        </section>
        <section className="approach-section" id="approach">
          <div className="container approach-grid">
            <div className="process-visual">
              <div className="process-caption">
                <span className="live-dot" /> THE WORK BEHIND THE WORDS
              </div>
              <div className="writing-window">
                <div className="writing-window-top">
                  <span>
                    <i />
                    <i />
                    <i />
                  </span>
                  <span>My next big idea</span>
                  <ScanLine size={15} />
                </div>
                <div className="writing-content">
                  <div className="serif-line">It starts with a thought.</div>
                  <p>
                    Then a question. A little research.
                    <br />A different perspective. A better draft.
                  </p>
                  <div className="writing-highlight">
                    And a voice that’s entirely your own.
                    <span className="typing-caret" />
                  </div>
                  <span className="writing-comment">
                    <MessageSquare size={13} /> Love this perspective. Keep
                    exploring!
                  </span>
                </div>
                <div className="writing-footer">
                  <span>
                    <span className="live-dot" /> Learning in progress
                  </span>
                  <span>Every draft matters.</span>
                </div>
              </div>
              <div className="process-steps">
                <span>
                  <Check size={13} /> Think
                </span>
                <span className="step-line" />
                <span>
                  <Check size={13} /> Create
                </span>
                <span className="step-line" />
                <span className="last-step">
                  <Sparkles size={13} /> Grow
                </span>
              </div>
            </div>
            <div className="approach-copy">
              <div className="eyebrow">
                PROTECT THE PROCESS. UNLOCK THE POTENTIAL.
              </div>
              <h2>
                There’s more to
                <br />
                learning than
                <br />
                <span>the final answer.</span>
              </h2>
              <p>
                Great learning happens along the way. Make room for curiosity,
                support responsible AI use, and see the thinking behind every
                submission.
              </p>
              <ul>
                <li>
                  <Check size={15} /> Put the writing process in perspective
                </li>
                <li>
                  <Check size={15} /> Create space for constructive
                  conversations
                </li>
                <li>
                  <Check size={15} /> Help students develop their own voice
                </li>
              </ul>
              <ArrowLink href="/dashboard/feedback">
                See a more complete picture
              </ArrowLink>
            </div>
          </div>
        </section>
        <section className="audience-section section-space container">
          <div className="section-intro">
            <div>
              <div className="eyebrow">DIFFERENT JOURNEYS. SHARED VALUES.</div>
              <h2>
                Your world.
                <br />
                <span>Our shared purpose.</span>
              </h2>
            </div>
            <div
              className="audience-tabs"
              role="tablist"
              aria-label="Education audience"
            >
              {["Higher education", "Secondary education", "Research"].map(
                (t) => (
                  <button
                    key={t}
                    role="tab"
                    aria-selected={audience === t}
                    className={audience === t ? "active" : ""}
                    onClick={() => setAudience(t)}
                  >
                    {t}
                  </button>
                ),
              )}
            </div>
          </div>
          <div className="audience-panel">
            <div className="audience-text">
              <span className="small-label">{audience.toUpperCase()}</span>
              <h3>
                {audience === "Research"
                  ? "Let good research make a lasting mark."
                  : audience === "Secondary education"
                    ? "Build good habits. Inspire great thinkers."
                    : "Build a culture of integrity. Not just a policy."}
              </h3>
              <p>
                {audience === "Research"
                  ? "Screen manuscripts, theses, and grant proposals for similarity and potential AI writing. Support original scholarship from first submission to publication."
                  : audience === "Secondary education"
                    ? "Give the next generation the skills to write with confidence, think independently, and navigate AI responsibly."
                    : "Bring students, educators, and institutions together around authentic work. Make assessment meaningful and give original thinking room to grow."}
              </p>
              <ArrowLink href="/dashboard/classes">
                Explore the possibilities
              </ArrowLink>
            </div>
            <div className="audience-art">
              <span className="art-caption">
                THINK INDEPENDENTLY. LEARN TOGETHER.
              </span>
              <div className="book book-one">
                <span>
                  Ideas
                  <br />
                  worth
                  <br />
                  <i>owning.</i>
                </span>
                <Fingerprint size={45} strokeWidth={1} />
              </div>
              <div className="book book-two">
                <span>
                  A new
                  <br />
                  perspective.
                </span>
                <span className="book-flower">✳</span>
              </div>
              <span className="art-spark">✦</span>
            </div>
          </div>
        </section>
        <section className="ecosystem-section container">
          <div className="eyebrow">
            ONE PURPOSE. A CONNECTED SET OF POSSIBILITIES.
          </div>
          <div className="ecosystem-grid">
            <article>
              <GraduationCap size={23} />
              <h3>Learning integrity</h3>
              <p>
                Support original work, responsible AI use, and meaningful
                feedback.
              </p>
              <span>Feedback Studio · Similarity · Clarity</span>
            </article>
            <article>
              <BookOpen size={23} />
              <h3>Research integrity</h3>
              <p>
                Review manuscripts, theses, and research for text similarity and
                AI writing.
              </p>
              <span>iThenticate</span>
            </article>
            <article>
              <ShieldCheck size={23} />
              <h3>Assessment with confidence</h3>
              <p>
                Bring consistency to grading and security to high-stakes digital
                exams.
              </p>
              <span>Gradescope · ExamSoft</span>
            </article>
          </div>
        </section>
        <section className="resources-section container" id="resources">
          <div className="section-intro compact">
            <div>
              <div className="eyebrow">A LITTLE KNOWLEDGE GOES A LONG WAY.</div>
              <h2>
                Keep your thinking <span>fresh.</span>
              </h2>
            </div>
            <ArrowLink href="/dashboard/resources">
              Explore all resources
            </ArrowLink>
          </div>
          <div className="resource-grid">
            {[
              {
                tag: "RESPONSIBLE AI",
                title: "AI in the classroom: start with the right questions.",
                className: "resource-ai",
                icon: Sparkles,
              },
              {
                tag: "ACADEMIC INTEGRITY",
                title: "A similarity score is the beginning of a conversation.",
                className: "resource-similarity",
                icon: Fingerprint,
              },
              {
                tag: "BETTER FEEDBACK",
                title: "Small comments. Meaningful steps forward.",
                className: "resource-feedback",
                icon: MessageSquare,
              },
            ].map(({ tag, title, className, icon: Icon }, i) => (
              <Link
                className="resource-card"
                to={`/dashboard/resources?article=${i}`}
                key={tag}
              >
                <div className={`resource-art ${className}`}>
                  <Icon size={65} strokeWidth={1} />
                  <span>
                    {i === 0
                      ? "Human + AI"
                      : i === 1
                        ? "Look closer."
                        : "Keep growing."}
                  </span>
                  <ArrowUpRight size={22} />
                </div>
                <span className="small-label">{tag} · 1 MIN READ</span>
                <h3>{title}</h3>
              </Link>
            ))}
          </div>
        </section>
        <section className="faq-section section-space container">
          <div>
            <div className="eyebrow">A FEW THINGS YOU MIGHT BE WONDERING.</div>
            <h2>
              Good questions.
              <br />
              <span>Clear answers.</span>
            </h2>
            <Link className="arrow-link" to="/dashboard/resources">
              Visit the resource library <ArrowUpRight size={17} />
            </Link>
          </div>
          <div className="faq-list">
            {questions.map(([q, a], i) => (
              <div
                className={`faq-item ${faq === i ? "expanded" : ""}`}
                key={q}
              >
                <button
                  aria-expanded={faq === i}
                  onClick={() => setFaq(faq === i ? null : i)}
                >
                  {q}
                  {faq === i ? <Minus size={18} /> : <Plus size={18} />}
                </button>
                {faq === i && <p>{a}</p>}
              </div>
            ))}
          </div>
        </section>
        <section className="cta-section container">
          <div>
            <div className="eyebrow">
              <span className="live-dot" /> THE NEXT CHAPTER STARTS WITH YOU.
            </div>
            <h2>
              Here’s to what
              <br />
              <span>only you can do.</span>
            </h2>
            <p>Original ideas. Confident educators. A world of possibility.</p>
            <Link to="/dashboard" className="button button-lime">
              Step inside the workspace <ArrowUpRight size={18} />
            </Link>
          </div>
          <div className="cta-art" aria-hidden="true">
            ✳
            <span>
              Stay curious.
              <br />
              Stay original.
            </span>
          </div>
        </section>
      </main>
      <footer className="site-footer container">
        <div className="footer-top">
          <div>
            <Brand />
            <p>
              Original thinking.
              <br />
              Lasting impact.
            </p>
          </div>
          <div>
            <span>EXPLORE</span>
            <Link to="#solutions">Our solutions</Link>
            <Link to="#approach">Our approach</Link>
            <Link to="#resources">Resources</Link>
          </div>
          <div>
            <span>WORKSPACE</span>
            <Link to="/dashboard">Dashboard</Link>
            <Link to="/dashboard/reports">Originality reports</Link>
            <Link to="/dashboard/classes">Classes & assignments</Link>
          </div>
          <div className="footer-news">
            <span>A BETTER FUTURE STARTS WITH LEARNING.</span>
            <p>
              For educators. For students.
              <br />
              For integrity.
            </p>
            <Globe2 size={22} />
          </div>
        </div>
        <div className="footer-bottom">
          <span>
            © 2026 · Independent Turnitin UI concept. Not affiliated with
            Turnitin, LLC.
          </span>
          <span>
            Designed for original minds. <span className="footer-star">✳</span>
          </span>
        </div>
      </footer>
    </div>
  );
}
