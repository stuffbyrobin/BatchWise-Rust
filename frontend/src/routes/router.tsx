import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from '../components/layout/AppShell'
import { ProtectedRoute } from '../auth/ProtectedRoute'
import { LoginPage } from '../features/auth/LoginPage'
import { RegisterPage } from '../features/auth/RegisterPage'
import { DashboardPage } from '../features/dashboard/DashboardPage'
import { LandingPage } from '../features/marketing/LandingPage'

// Inventory
import { InventoryListPage } from '../features/inventory/InventoryListPage'
import { InventoryCreatePage } from '../features/inventory/InventoryCreatePage'
import { InventoryDetailPage } from '../features/inventory/InventoryDetailPage'
import { InventorySummaryPage } from '../features/inventory/InventorySummaryPage'
import { InventoryMovementsPage } from '../features/inventory/InventoryMovementsPage'
import { InventoryImportPage } from '../features/inventory/InventoryImportPage'

// Recipes
import RecipesListPage from '../features/recipes/RecipesListPage'
import RecipeEditorPage from '../features/recipes/RecipeEditorPage'
import RecipeImportPage from '../features/recipes/RecipeImportPage'

// Library
import { StylesPage } from '../features/library/StylesPage'
import { EquipmentProfilesPage } from '../features/library/EquipmentProfilesPage'
import { MashProfilesPage } from '../features/library/MashProfilesPage'
import { YeastsPage } from '../features/library/YeastsPage'
import { LibraryFermentablesPage } from '../features/library/LibraryFermentablesPage'

// Batches
import { BatchesListPage } from '../features/batches/BatchesListPage'
import { BatchCreatePage } from '../features/batches/BatchCreatePage'
import FermentersPage from '../features/fermenters/FermentersPage'
import FermenterSchedulePage from '../features/fermenters/FermenterSchedulePage'
import { BatchDetailPage } from '../features/batches/BatchDetailPage'
import { BatchImportPage } from '../features/batches/BatchImportPage'

// Calendar
import { CalendarPage } from '../features/calendar/CalendarPage'

// Yeast kinetics
import { YeastKineticsPage } from '../features/yeast-kinetics/YeastKineticsPage'

// Account
import { AccountPage } from '../features/account/AccountPage'

// Water chemistry
import { WaterProfilesPage } from '../features/water/WaterProfilesPage'
import { WaterCalculatorPage } from '../features/water/WaterCalculatorPage'
import { WaterAdjustmentsPage } from '../features/water/WaterAdjustmentsPage'

// Beer Duty
import { DutyReturnsPage } from '../features/duty/DutyReturnsPage'

// Label Records
import { LabelRecordsPage } from '../features/labels/LabelRecordsPage'

// Label & Print Design
import { LabelDesignsPage } from '../features/labels/LabelDesignsPage'
import { LabelDesignEditorPage } from '../features/labels/LabelDesignEditorPage'
import { BrandProfilesPage } from '../features/labels/BrandProfilesPage'

// Packaging & Traceability
import PackagingRunsPage from '../features/packaging/PackagingRunsPage'
import DistributionMovementsPage from '../features/packaging/DistributionMovementsPage'
import TraceabilityPage from '../features/traceability/TraceabilityPage'

// Procurement
import SuppliersPage from '../features/procurement/SuppliersPage'
import PurchaseOrdersPage from '../features/procurement/PurchaseOrdersPage'

// Yeast Bank
import YeastBankPage from '../features/yeast-bank/YeastBankPage'

// Fermentation
import { FermentationPage } from '../features/fermentation/FermentationPage'

// Equipment maintenance
import EquipmentPage from '../features/equipment/EquipmentPage'
import MaintenanceDuePage from '../features/equipment/MaintenanceDuePage'

// Compliance Audit
import ComplianceAuditPage from '../features/compliance/ComplianceAuditPage'

// Phase 07d — reporting & container assets
import { ContainerAssetsListPage } from '../features/reporting/ContainerAssetsListPage'
import { ContainerAssetDetailPage } from '../features/reporting/ContainerAssetDetailPage'
import { ContainerAssetQRPage } from '../features/reporting/ContainerAssetQRPage'
import { CostRatesPage } from '../features/reporting/CostRatesPage'
import { BatchCostsPage } from '../features/reporting/BatchCostsPage'
import { CostReportsPage } from '../features/reporting/CostReportsPage'

const router = createBrowserRouter([
  { path: '/', element: <LandingPage /> },
  { path: '/login', element: <LoginPage /> },
  { path: '/register', element: <RegisterPage /> },
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
