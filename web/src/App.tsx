import React from 'react'
import { BrowserRouter, Route, Routes } from 'react-router-dom'
import { createGlobalStyle } from 'styled-components'
import { Edit } from './pages/Edit'
import { View } from './pages/View'

export const App: React.FC = () => (
  <>
    <BrowserRouter>
      <Routes>
        <Route path='/' element={<Edit />} />
        <Route path='/:pageId' element={<View />} />
      </Routes>
    </BrowserRouter>
  </>

)