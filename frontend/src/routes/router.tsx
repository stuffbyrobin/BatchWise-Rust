import { lazy, type ComponentType } from 'react'
import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from '../components/layout/AppShell'
import { ProtectedRoute } from '../auth/ProtectedRoute'
import { LoginPage } from '../features/auth/LoginPage'
import { RegisterPage } from '../features/auth/RegisterPage'
import { AcceptInvitationPage } from '../features/auth/AcceptInvitationPage'
import { LandingPage } from '../features/marketing/LandingPage'

// Protected pages are split into their own chunks and load on first visit;
// the public landing, login, register and invite pages stay in the entry chunk.
function named<K extends string, M extends Record<K, ComponentType>>(load: () => Promise<M>, name: K) {
  return lazy(() => load().then((m) => ({ default: m[name] })))
}

// Dashboard
const DashboardPage = named(() => import('../features/dashboard/DashboardPage'), 'DashboardPage')

// Inventory
const InventoryListPage = named(() => import('../features/inventory/InventoryListPage'), 'InventoryListPage')
const InventoryCreatePage = named(() => import('../features/inventory/InventoryCreatePage'), 'InventoryCreatePage')
const InventoryDetailPage = named(() => import('../features/inventory/InventoryDetailPage'), 'InventoryDetailPage')
const InventorySummaryPage = named(() => import('../features/inventory/InventorySummaryPage'), 'InventorySummaryPage')
const InventoryMovementsPage = named(() => import('../features/inventory/InventoryMovementsPage'), 'InventoryMovementsPage')
const InventoryImportPage = named(() => import('../features/inventory/InventoryImportPage'), 'InventoryImportPage')

// Recipes
const RecipesListPage = lazy(() => import('../features/recipes/RecipesListPage'))
const RecipeEditorPage = lazy(() => import('../features/recipes/RecipeEditorPage'))
const RecipeImportPage = lazy(() => import('../features/recipes/RecipeImportPage'))

// Library
const StylesPage = named(() => import('../features/library/StylesPage'), 'StylesPage')
const EquipmentProfilesPage = named(() => import('../features/library/EquipmentProfilesPage'), 'EquipmentProfilesPage')
const MashProfilesPage = named(() => import('../features/library/MashProfilesPage'), 'MashProfilesPage')
const YeastsPage = named(() => import('../features/library/YeastsPage'), 'YeastsPage')
const LibraryFermentablesPage = named(() => import('../features/library/LibraryFermentablesPage'), 'LibraryFermentablesPage')

// Batches
const BatchesListPage = named(() => import('../features/batches/BatchesListPage'), 'BatchesListPage')
const BatchCreatePage = named(() => import('../features/batches/BatchCreatePage'), 'BatchCreatePage')
const FermentersPage = lazy(() => import('../features/fermenters/FermentersPage'))
const FermenterSchedulePage = lazy(() => import('../features/fermenters/FermenterSchedulePage'))
const BatchDetailPage = named(() => import('../features/batches/BatchDetailPage'), 'BatchDetailPage')
const BatchImportPage = named(() => import('../features/batches/BatchImportPage'), 'BatchImportPage')

// Calendar
const CalendarPage = named(() => import('../features/calendar/CalendarPage'), 'CalendarPage')

// Yeast kinetics
const YeastKineticsPage = named(() => import('../features/yeast-kinetics/YeastKineticsPage'), 'YeastKineticsPage')

// Account
const AccountPage = named(() => import('../features/account/AccountPage'), 'AccountPage')

// Water chemistry
const WaterProfilesPage = named(() => import('../features/water/WaterProfilesPage'), 'WaterProfilesPage')
const WaterCalculatorPage = named(() => import('../features/water/WaterCalculatorPage'), 'WaterCalculatorPage')
const WaterAdjustmentsPage = named(() => import('../features/water/WaterAdjustmentsPage'), 'WaterAdjustmentsPage')

// Beer Duty
const DutyReturnsPage = named(() => import('../features/duty/DutyReturnsPage'), 'DutyReturnsPage')

// Label Records
const LabelRecordsPage = named(() => import('../features/labels/LabelRecordsPage'), 'LabelRecordsPage')

// Label & Print Design
const LabelDesignsPage = named(() => import('../features/labels/LabelDesignsPage'), 'LabelDesignsPage')
const LabelDesignEditorPage = named(() => import('../features/labels/LabelDesignEditorPage'), 'LabelDesignEditorPage')
const BrandProfilesPage = named(() => import('../features/labels/BrandProfilesPage'), 'BrandProfilesPage')

// Packaging & Traceability
const PackagingRunsPage = lazy(() => import('../features/packaging/PackagingRunsPage'))
const DistributionMovementsPage = lazy(() => import('../features/packaging/DistributionMovementsPage'))
const TraceabilityPage = lazy(() => import('../features/traceability/TraceabilityPage'))

// Procurement
const SuppliersPage = lazy(() => import('../features/procurement/SuppliersPage'))
const PurchaseOrdersPage = lazy(() => import('../features/procurement/PurchaseOrdersPage'))

// Yeast Bank
const YeastBankPage = lazy(() => import('../features/yeast-bank/YeastBankPage'))

// Fermentation
const FermentationPage = named(() => import('../features/fermentation/FermentationPage'), 'FermentationPage')

// Equipment maintenance
const EquipmentPage = lazy(() => import('../features/equipment/EquipmentPage'))
const MaintenanceDuePage = lazy(() => import('../features/equipment/MaintenanceDuePage'))

// Compliance Audit
const ComplianceAuditPage = lazy(() => import('../features/compliance/ComplianceAuditPage'))

// Members
const MembersPage = named(() => import('../features/members/MembersPage'), 'MembersPage')

// Phase 07d — reporting & container assets
const ContainerAssetsListPage = named(() => import('../features/reporting/ContainerAssetsListPage'), 'ContainerAssetsListPage')
const ContainerAssetDetailPage = named(() => import('../features/reporting/ContainerAssetDetailPage'), 'ContainerAssetDetailPage')
const ContainerAssetQRPage = named(() => import('../features/reporting/ContainerAssetQRPage'), 'ContainerAssetQRPage')
const CostRatesPage = named(() => import('../features/reporting/CostRatesPage'), 'CostRatesPage')
const BatchCostsPage = named(() => import('../features/reporting/BatchCostsPage'), 'BatchCostsPage')
const CostReportsPage = named(() => import('../features/reporting/CostReportsPage'), 'CostReportsPage')

const router = createBrowserRouter([
  { path: '/', element: <LandingPage /> },
  { path: '/login', element: <LoginPage /> },
  { path: '/register', element: <RegisterPage /> },
  { path: '/invite', element: <AcceptInvitationPage /> },
  {
    element: (
      <ProtectedRoute>
        <AppShell />
      </ProtectedRoute>
    ),
    children: [
      { path: 'app', element: <DashboardPage /> },

      // Inventory
      { path: 'inventory', element: <InventoryListPage /> },
      { path: 'inventory/new', element: <InventoryCreatePage /> },
      { path: 'inventory/import', element: <InventoryImportPage /> },
      { path: 'inventory/summary', element: <InventorySummaryPage /> },
      { path: 'inventory/movements', element: <InventoryMovementsPage /> },
      { path: 'inventory/:id', element: <InventoryDetailPage /> },

      // Recipes
      { path: 'recipes', element: <RecipesListPage /> },
      { path: 'recipes/new', element: <RecipeEditorPage /> },
      { path: 'recipes/import', element: <RecipeImportPage /> },
      { path: 'recipes/:id', element: <RecipeEditorPage /> },

      // Library
      { path: 'library/styles', element: <StylesPage /> },
      { path: 'library/equipment-profiles', element: <EquipmentProfilesPage /> },
      { path: 'library/mash-profiles', element: <MashProfilesPage /> },
      { path: 'library/yeasts', element: <YeastsPage /> },
      { path: 'library/fermentables', element: <LibraryFermentablesPage /> },

      // Batches
      { path: 'batches', element: <BatchesListPage /> },
      { path: 'batches/new', element: <BatchCreatePage /> },
      { path: 'batches/import', element: <BatchImportPage /> },
      { path: 'batches/:id', element: <BatchDetailPage /> },
      { path: 'batches/:batchId/fermentation', element: <FermentationPage /> },

      // Fermenters & schedule (Gantt)
      { path: 'fermenters', element: <FermentersPage /> },
      { path: 'fermenters/schedule', element: <FermenterSchedulePage /> },

      // Calendar & yeast kinetics
      { path: 'calendar', element: <CalendarPage /> },
      { path: 'yeast-kinetics', element: <YeastKineticsPage /> },
      { path: 'yeast-bank', element: <YeastBankPage /> },

      // Account settings
      { path: 'account', element: <AccountPage /> },

      // Water chemistry
      { path: 'water/profiles', element: <WaterProfilesPage /> },
      { path: 'water/calculator', element: <WaterCalculatorPage /> },
      { path: 'water/adjustments', element: <WaterAdjustmentsPage /> },

      // Beer Duty
      { path: 'duty', element: <DutyReturnsPage /> },

      // Label Records
      { path: 'labels', element: <LabelRecordsPage /> },

      // Label & Print Design
      { path: 'label-design', element: <LabelDesignsPage /> },
      { path: 'label-design/new', element: <LabelDesignEditorPage /> },
      { path: 'label-design/brands', element: <BrandProfilesPage /> },
      { path: 'label-design/:id', element: <LabelDesignEditorPage /> },

      // Procurement
      { path: 'suppliers', element: <SuppliersPage /> },
      { path: 'purchase-orders', element: <PurchaseOrdersPage /> },
      { path: 'equipment', element: <EquipmentPage /> },
      { path: 'maintenance-due', element: <MaintenanceDuePage /> },

      // Packaging & Traceability
      { path: 'packaging-runs', element: <PackagingRunsPage /> },
      { path: 'distribution-movements', element: <DistributionMovementsPage /> },
      { path: 'traceability', element: <TraceabilityPage /> },
      { path: 'compliance-audit', element: <ComplianceAuditPage /> },
      { path: 'members', element: <MembersPage /> },

      // Phase 07d — reporting & container assets
      { path: 'container-assets', element: <ContainerAssetsListPage /> },
      { path: 'container-assets/:id', element: <ContainerAssetDetailPage /> },
      { path: 'container-assets/:id/qr', element: <ContainerAssetQRPage /> },
      { path: 'cost-rates', element: <CostRatesPage /> },
      { path: 'batch-costs', element: <BatchCostsPage /> },
      { path: 'cost-reports', element: <CostReportsPage /> },
    ],
  },
])

export default router
