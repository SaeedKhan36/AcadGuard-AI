import {
  ArrowLeft,
  ArrowUpRight,
  Check,
  GraduationCap,
  ShieldCheck,
} from "lucide-react";
import { Brand } from "../components/Brand";
export default function Login() {
  return (
    <main className="login-page">
      <div className="login-side">
        <Brand light />
        <div>
          <span className="eyebrow">YOUR NEXT CHAPTER</span>
          <h1>
            Good to
            <br />
            have <em>you</em>
            <br />
            here.
          </h1>
          <p>
            A little clarity. A lot of possibility.
            <br />
            Let’s make room for original thinking.
          </p>
        </div>
        <span className="login-side-bottom">
          For educators. For students. For integrity. <span>✳</span>
        </span>
      </div>
      <div className="login-main">
        <a className="back-link" href="#/">
          <ArrowLeft size={15} /> Back to home
        </a>
        <div className="login-card">
          <span className="login-illustration">
            <GraduationCap size={32} />
          </span>
          <div className="eyebrow">A SPACE FOR ORIGINAL MINDS</div>
          <h2>Welcome to your workspace.</h2>
          <p>Take a look around. Your next perspective is waiting.</p>
          <div className="demo-notice">
            <ShieldCheck size={20} />
            <div>
              <strong>You’re exploring a UI concept</strong>
              <span>
                Sample documents and reports. No account or password needed.
              </span>
            </div>
          </div>
          <a href="#/app" className="button button-dark">
            Enter demo workspace <ArrowUpRight size={18} />
          </a>
          <div className="login-checks">
            <span>
              <Check size={13} /> No sign-up
            </span>
            <span>
              <Check size={13} /> Illustrative data
            </span>
          </div>
        </div>
        <span className="login-disclaimer">
          Independent prototype · Not an official Turnitin service
        </span>
      </div>
    </main>
  );
}
