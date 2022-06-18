import SyntaxHighlighter from 'react-syntax-highlighter';
import { atomOneDark } from 'react-syntax-highlighter/dist/esm/styles/hljs';
import { useNavigate, useParams } from "react-router-dom"
import { Menu } from "../components/Menu";
import { MenuButton } from '../components/MenuButton';
import { useEffect, useState } from 'react';
import axios from 'axios';
import { API_URL } from '../constants';




export const View: React.FC = () => {
    const { pageId } = useParams()
    const [text, setText] = useState("");

    const navigate = useNavigate()

    useEffect(() => {
        axios.get(`${API_URL}/${pageId}`)
            .then(({data: text}) => setText(text))
            .catch(err => navigate('/'))

    }, [pageId])

    const newEdit = () => {
        navigate('/')
    }

    const forkEdit = () => {
        navigate('/', {
            state: {
                text
            }
        })
    }

    return (
        <div>
            <Menu>
                <MenuButton onClick={forkEdit}>Fork</MenuButton>
                <MenuButton onClick={newEdit}>New</MenuButton>
            </Menu>
            <div>
                <SyntaxHighlighter showLineNumbers style={atomOneDark}>
                    {text}
                </SyntaxHighlighter>
            </div>
        </div>
    )
}