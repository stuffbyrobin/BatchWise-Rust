import { useState, type FormEvent } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '../../auth/useAuth';
import { safeRedirectPath } from '../../auth/redirect';
import { APIError } from '../../api/error';
import { AuthLayout, authAlertCls, authButtonCls, authInputCls } from '../../components/layout/AuthLayout';

export function LoginPage() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const auth = useAuth();

  const handleLogin = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await auth.login(email, password);
      navigate(safeRedirectPath(searchParams.get('from')));
    } catch (err) {
      setError(err instanceof APIError ? err.message : 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <AuthLayout title="Welcome back" subtitle="Sign in to keep brewing.">
      {error && (
        <div className={authAlertCls}>{error}</div>
      )}
      <form className="space-y-4" onSubmit={handleLogin}>
        <div>
          <label htmlFor="login-email" className="block text-sm font-medium text-(--lp-ink) mb-1">Email</label>
          <input
            id="login-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className={authInputCls}
            autoComplete="email" required
          />
        </div>
        <div>
          <label htmlFor="login-password" className="block text-sm font-medium text-(--lp-ink) mb-1">Password</label>
          <input
            id="login-password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className={authInputCls}
            autoComplete="current-password" required
          />
        </div>
        <button type="submit" disabled={loading} className={authButtonCls}>
          {loading ? 'Signing in...' : 'Sign in'}
        </button>
      </form>
      <div className="mt-6 text-center text-[13px] text-(--lp-muted)">
        <Link to="/register" className="text-(--lp-malt-deep) font-semibold no-underline hover:text-(--lp-ink) transition-colors">
          Create an account
        </Link>
      </div>
    </AuthLayout>
  );
}
