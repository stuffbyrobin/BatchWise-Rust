import { Suspense } from 'react'
import { Outlet } from 'react-router-dom'
import { Sidebar } from './Sidebar'
import { TopBar } from './TopBar'
import { Spinner } from '../feedback/Spinner'
import { RouteAccess } from './RouteAccess'

/**
 * App layout shell with Sidebar, TopBar, and the routed page via Outlet.
 * Pages are lazily loaded, so the Outlet sits in a Suspense boundary that shows a spinner while a page chunk loads.
 * RouteAccess applies the user's role to the page first.
 */
export function AppShell() {
  return (
    <div className="flex h-screen overflow-hidden" style={{ background: 'var(--color-bg)' }}>
      <Sidebar />
      <div className="flex flex-col flex-1 overflow-hidden">
        <TopBar />
        <main className="flex-1 overflow-y-auto p-6">
          <RouteAccess>
            <Suspense
              fallback={
                <div className="flex justify-center py-12">
                  <Spinner />
                </div>
              }
            >
              <Outlet />
            </Suspense>
          </RouteAccess>
        </main>
      </div>
    </div>
  )
}
