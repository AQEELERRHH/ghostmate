import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout }     from './components/Layout';
import { Home }       from './pages/Home';
import { Register }   from './pages/Register';
import { Challenges } from './pages/Challenges';
import { LiveGame }   from './pages/LiveGame';
import { Profile }    from './pages/Profile';
import { Agents }     from './pages/Agents';

export function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route index                  element={<Home />} />
          <Route path="live"            element={<LiveGame />} />
          <Route path="game"            element={<LiveGame />} />
          <Route path="agents"          element={<Agents />} />
          <Route path="challenges"      element={<Challenges />} />
          <Route path="register"        element={<Register />} />
          <Route path="profile"         element={<Profile />} />
          <Route path="profile/:prefix" element={<Profile />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
