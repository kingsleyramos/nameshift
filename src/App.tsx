import { useEffect, useState } from 'react';
import MainWindow from './components/shell/MainWindow';
import HelpWindow from './components/help/HelpWindow';
import SettingsWindow from './components/settings/SettingsWindow';
import { startAppStore } from './state/appState';

function routeFromHash(): 'main' | 'help' | 'settings' {
  const hash = window.location.hash;
  if (hash.startsWith('#/help')) return 'help';
  if (hash.startsWith('#/settings')) return 'settings';
  return 'main';
}

export default function App() {
  const [route, setRoute] = useState(routeFromHash);
  useEffect(() => {
    const onHashChange = () => {
      setRoute(routeFromHash());
    };
    window.addEventListener('hashchange', onHashChange);
    return () => {
      window.removeEventListener('hashchange', onHashChange);
    };
  }, []);
  useEffect(() => {
    void startAppStore();
  }, []);

  if (route === 'help') return <HelpWindow />;
  if (route === 'settings') return <SettingsWindow />;
  return <MainWindow />;
}
