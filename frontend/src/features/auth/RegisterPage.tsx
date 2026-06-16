import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../../auth/useAuth';
import { APIError } from '../../api/error';

export function RegisterPage() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [tenantName, setTenantName] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const auth = useAuth();

  const handleRegister = async () => {
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
    <div className="p-6 max-w-md mx-auto">
      <h1 className="text-2xl font-bold mb-6">Create Account</h1>
      {error && (
        <div className="bg-red-100 text-red-700 p-3 rounded mb-4">{error}</div>
      )}
      <div className="space-y-4">
        <div>
          <label htmlFor="reg-email" className="block text-sm font-medium mb-1">Email</label>
          <input
            id="reg-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="w-full p-2 border rounded"
          />
        </div>
        <div>
          <label htmlFor="reg-display-name" className="block text-sm font-medium mb-1">Your Name</label>
          <input
            id="reg-display-name"
            type="text"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            className="w-full p-2 border rounded"
          />
        </div>
        <div>
          <label htmlFor="reg-password" className="block text-sm font-medium mb-1">Password</label>
          <input
            id="reg-password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="w-full p-2 border rounded"
          />
        </div>
        <div>
          <label htmlFor="reg-tenant" className="block text-sm font-medium mb-1">Brewery Name</label>
          <input
            id="reg-tenant"
            type="text"
            value={tenantName}
            onChange={(e) => setTenantName(e.target.value)}
            className="w-full p-2 border rounded"
          />
        </div>
        <button
          onClick={handleRegister}
          disabled={loading}
          className="w-full bg-blue-600 text-white p-2 rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? 'Creating account...' : 'Create account'}
        </button>
      </div>
      <div className="mt-4 text-center">
        <Link to="/login" className="text-blue-600 hover:underline">
          Already have an account? Sign in
        </Link>
      </div>
    </div>
  );
}
