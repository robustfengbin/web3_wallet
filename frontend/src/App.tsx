import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider } from './hooks/useAuth';
import { ProtectedRoute } from './components/Layout';
import {
  Login,
  Dashboard,
  History,
  Settings,
  EthereumWallets,
  EthereumTransfer,
  EthereumRpcSettings,
  ZcashWallets,
  ZcashTransfer,
  ZcashRpcSettings,
} from './pages';

function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<Login />} />
          <Route
            path="/"
            element={
              <ProtectedRoute>
                <Dashboard />
              </ProtectedRoute>
            }
          />
          {/* Legacy routes - redirect to Ethereum */}
          <Route path="/wallets" element={<Navigate to="/ethereum/wallets" replace />} />
          <Route path="/transfer" element={<Navigate to="/ethereum/transfer" replace />} />
          <Route
            path="/history"
            element={
              <ProtectedRoute>
                <History />
              </ProtectedRoute>
            }
          />
          <Route
            path="/settings"
            element={
              <ProtectedRoute>
                <Settings />
              </ProtectedRoute>
            }
          />

          {/* Ethereum Routes */}
          <Route
            path="/ethereum/wallets"
            element={
              <ProtectedRoute>
                <EthereumWallets />
              </ProtectedRoute>
            }
          />
          <Route
            path="/ethereum/transfer"
            element={
              <ProtectedRoute>
                <EthereumTransfer />
              </ProtectedRoute>
            }
          />
          <Route
            path="/ethereum/rpc"
            element={
              <ProtectedRoute>
                <EthereumRpcSettings />
              </ProtectedRoute>
            }
          />

          {/* Zcash Routes */}
          <Route
            path="/zcash/wallets"
            element={
              <ProtectedRoute>
                <ZcashWallets />
              </ProtectedRoute>
            }
          />
          <Route
            path="/zcash/transfer"
            element={
              <ProtectedRoute>
                <ZcashTransfer />
              </ProtectedRoute>
            }
          />
          <Route
            path="/zcash/rpc"
            element={
              <ProtectedRoute>
                <ZcashRpcSettings />
              </ProtectedRoute>
            }
          />

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  );
}

export default App;
