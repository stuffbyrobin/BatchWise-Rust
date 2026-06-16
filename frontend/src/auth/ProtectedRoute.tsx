import { Navigate, useLocation } from 'react-router-dom';
import { useAuth } from './useAuth';

export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, isLoading } = useAuth();
  const location = useLocation();

  if (isLoading) return <div>Loading...</div>;
  if (!user)
    return (
      <Navigate
        to={'/login?from=' + encodeURIComponent(location.pathname)}
        replace
      />
    );
  return <>{children}</>;
}
