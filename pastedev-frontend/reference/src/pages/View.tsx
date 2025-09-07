import SyntaxHighlighter from 'react-syntax-highlighter';
import { atomOneDark } from 'react-syntax-highlighter/dist/esm/styles/hljs';
import { useLocation, useNavigate, useParams } from "react-router-dom"
import { Menu } from "../components/Menu";
import { MenuButton } from '../components/MenuButton';
import { useEffect, useState } from 'react';
import { API_URL } from '../constants';


export const View: React.FC = () => {
    const { pageId } = useParams()
    const [text, setText] = useState("");

    const navigate = useNavigate()

    useEffect(() => {
        const fetchText = async () => {
            const resp = await fetch(`${API_URL}/${pageId}`)
            const text = await resp.text()

            if (!resp.ok) throw new Error(`Status code: ${resp.status}`)

            setText(text)
        }

        fetchText().catch(err => navigate('/'))

    }, [pageId])

    const openRaw = () => {
        window.open(`${API_URL}/${pageId}`)
    }

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
                <MenuButton onClick={openRaw}>Raw</MenuButton>
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