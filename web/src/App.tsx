import React from 'react'
import { BrowserRouter, Route, Routes } from 'react-router-dom'
import { createGlobalStyle } from 'styled-components'
import { Edit } from './pages/Edit'
import { View } from './pages/View'

const GlobalStyle = createGlobalStyle`
  body {
    background-color: #282c34;
  }
`

export const App: React.FC = () => (
  <>
    <GlobalStyle />
    <BrowserRouter>
      <Routes>
        <Route path='/' element={<Edit />} />
        <Route path='/:pageId' element={<View />} />
      </Routes>
    </BrowserRouter>
  </>

)