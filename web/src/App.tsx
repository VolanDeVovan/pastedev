import React from 'react'
import { BrowserRouter, Route, Routes } from 'react-router-dom'
import { createGlobalStyle } from 'styled-components'
import { Edit } from './pages/Edit'
import { View } from './pages/View'

const GlobalStyle = createGlobalStyle`
  body {
    padding: 0;
    margin: 0;
  }

  * {
    box-sizing: border-box;
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