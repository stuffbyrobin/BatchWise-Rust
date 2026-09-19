import { useState, type FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../../auth/useAuth';
import { PASSWORD_HINT, PASSWORD_MIN_LENGTH, passwordProblem } from '../../auth/password';
import { APIError } from '../../api/error';
import { AuthLayout, authAlertCls, authButtonCls, authInputCls } from '../../components/layout/AuthLayout';

export function RegisterPage() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [tenantName, setTenantName] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const auth = useAuth();

  const handleRegister = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const problem = passwordProblem(password);
    if (problem) {
      setError(problem);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await auth.register(email, password, displayName, tenantName);
      navigate('/app');
    } catch (err) {
      setError(err instanceof APIError ? err.message : 'Registration failed. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <AuthLayout title="Start brewing smarter" subtitle="Free forever for a single brewery. No card required.">
      {error && (
        <div className={authAlertCls}>{error}</div>
      )}
      <form className="space-y-4" onSubmit={handleRegister}>
        <div>
          <label htmlFor="reg-email" className="block text-sm font-medium text-(--lp-ink) mb-1">Email</label>
          <input
            id="reg-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className={authInputCls}
            autoComplete="email" required
          />
        </div>
        <div>
          <label htmlFor="reg-display-name" className="block text-sm font-medium text-(--lp-ink) mb-1">Your Name</label>
          <input
            id="reg-display-name"
            type="text"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            className={authInputCls}
            autoComplete="name" required
          />
        </div>
        <div>
          <label htmlFor="reg-password" className="block text-sm font-medium text-(--lp-ink) mb-1">Password</label>
          <input
            id="reg-password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className={authInputCls}
            autoComplete="new-password" required
            minLength={PASSWORD_MIN_LENGTH}
            aria-describedby="reg-password-hint"
          />
          <p id="reg-password-hint" className="mt-1 text-xs text-(--lp-muted)">{PASSWORD_HINT}</p>
        </div>
        <div>
          <label htmlFor="reg-tenant" className="block text-sm font-medium text-(--lp-ink) mb-1">Brewery Name</label>
          <input
            id="reg-tenant"
            type="text"
            value={tenantName}
            onChange={(e) => setTenantName(e.target.value)}
            className={authInputCls}
            autoComplete="organization" required
          />
        </div>
        <button type="submit" disabled={loading} className={authButtonCls}>
          {loading ? 'Creating account...' : 'Create account'}
        </button>
      </form>
      <div className="mt-6 text-center text-[13px] text-(--lp-muted)">
        <Link to="/login" className="text-(--lp-malt-deep) font-semibold no-underline hover:text-(--lp-ink) transition-colors">
          Already have an account? Sign in
        </Link>
      </div>
    </AuthLayout>
  );
}
