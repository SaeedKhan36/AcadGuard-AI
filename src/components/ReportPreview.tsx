import {
  ArrowUpRight,
  Check,
  FileText,
  Fingerprint,
  Sparkles,
  ShieldCheck,
} from "lucide-react";
export function ReportPreview() {
  return (
    <div className="preview-scene" aria-label="Illustrative originality report">
      <div className="orbit orbit-one" />
      <div className="orbit orbit-two" />
      <div className="scene-cross cross-one">+</div>
      <div className="scene-cross cross-two">+</div>
      <div className="preview-label">
        <span className="live-dot" /> A LITTLE CLARITY GOES A LONG WAY
      </div>
      <div className="report-float">
        <div className="report-top">
          <div className="report-file">
            <span className="file-icon">
              <FileText size={19} />
            </span>
            <div>
              <strong>A perspective of my own</strong>
              <span>Essay · 2,450 words</span>
            </div>
          </div>
          <span className="report-menu">···</span>
        </div>
        <div className="report-body">
          <div className="report-eyebrow">
            ORIGINALITY AT A GLANCE <ArrowUpRight size={14} />
          </div>
          <div className="report-score">
            <div className="score-ring">
              <div>
                <strong>
                  92<span>%</span>
                </strong>
                <small>Original writing</small>
              </div>
            </div>
            <div className="score-info">
              <span>
                <i className="dot green" />
                Original content <b>92%</b>
              </span>
              <span>
                <i className="dot yellow" />
                Matched text <b>8%</b>
              </span>
              <p>
                <ShieldCheck size={13} /> A stronger starting point.
              </p>
            </div>
          </div>
          <div className="paper-lines">
            <span />
            <span />
            <span className="highlight-line" />
            <span />
            <span />
            <span className="short-line" />
          </div>
          <div className="report-bottom">
            <span>
              <span className="tiny-avatar">AL</span>Alex’s submission
            </span>
            <span>
              Report complete <Check size={12} />
            </span>
          </div>
        </div>
      </div>
      <div className="float-badge ai-badge">
        <span className="badge-icon purple">
          <Sparkles size={20} />
        </span>
        <div>
          <strong>More insight. Less guesswork.</strong>
          <span>AI writing insights, in context.</span>
        </div>
        <span className="badge-check">
          <Check size={12} />
        </span>
      </div>
      <div className="float-badge originality-badge">
        <span className="badge-icon sage">
          <Fingerprint size={21} />
        </span>
        <div>
          <strong>Every idea has a story.</strong>
          <span>Help original thinking shine.</span>
        </div>
      </div>
      <div className="scene-caption">
        <span>Made for human potential.</span>
        <span className="caption-arrow">↗</span>
      </div>
    </div>
  );
}
