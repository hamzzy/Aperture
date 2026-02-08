import { Toaster } from "@/components/ui/toaster";
import { Toaster as Sonner } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Phase8Provider } from "@/contexts/Phase8Context";
import Dashboard from "./pages/Dashboard";
import FlamegraphPage from "./pages/FlamegraphPage";
import TopFunctionsPage from "./pages/TopFunctionsPage";
import ComparisonPage from "./pages/ComparisonPage";
import TimelinePage from "./pages/TimelinePage";
import SettingsPage from "./pages/SettingsPage";
import NotFound from "./pages/NotFound";

const queryClient = new QueryClient();

const App = () => (
  <QueryClientProvider client={queryClient}>
    <Phase8Provider>
      <TooltipProvider>
        <Toaster />
        <Sonner />
        <BrowserRouter>
          <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/flamegraph" element={<FlamegraphPage />} />
          <Route path="/functions" element={<TopFunctionsPage />} />
          <Route path="/comparison" element={<ComparisonPage />} />
          <Route path="/timeline" element={<TimelinePage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<NotFound />} />
          </Routes>
        </BrowserRouter>
      </TooltipProvider>
    </Phase8Provider>
  </QueryClientProvider>
);

export default App;
