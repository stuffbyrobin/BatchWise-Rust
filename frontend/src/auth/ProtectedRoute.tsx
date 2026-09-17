import { Navigate, useLocation } from 'react-router-dom';
import { useAuth } from './useAuth';

/**
 * Redirects to /login?from=<path+query> when there is no user.
 */
export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, isLoading } = useAuth();
  const location = useLocation();

  if (isLoading) return <div>Loading...</div>;
  if (!user)
    return (
      <Navigate
        to={'/login?from=' + encodeURIComponent(location.pathname + location.search)}
        replace
      />
    );
  return <>{children}</>;
}
