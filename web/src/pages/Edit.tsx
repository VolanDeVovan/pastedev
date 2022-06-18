import { useRef, useState } from "react"
import { useLocation, useNavigate } from "react-router-dom"
import styled from "styled-components"
import { Menu } from "../components/Menu"
import { MenuButton } from "../components/MenuButton"
import { API_URL } from "../constants"


const InputSymbol = styled.div`
    z-index: -1000;
    position: absolute;

    top: 20px;
    left: 15px;
    width: 30px;

    color: #abb2bf;
    font-family: monospace;

`

const Textarea = styled.textarea`
    position: absolute;

    padding: 20px 30px;

    width: 100%;
    height: 95%;

    border: none;
    background: none;
    outline: none;

    color: white;
`


export const Edit: React.FC = () => {
    const location = useLocation()
    const navigate = useNavigate()

    const ref: any = useRef(null)

    // Reset text state
    history.replaceState(null, document.title)

    const saveButton = async () => {
        if (!ref?.current?.value) return

        const resp = await fetch(API_URL, {
            method: 'POST',
            body: ref.current.value,
        })

        const { snippet_id } = await resp.json()

        navigate(snippet_id)
    }

    return (
        <div>
            <Menu>
                <MenuButton onClick={saveButton}>Save</MenuButton>
            </Menu>
            <div>
                <InputSymbol>
                    {">"}
                </InputSymbol>
                <Textarea ref={ref} defaultValue={(location.state as any)?.text || ''} />
            </div>

        </div>
    )
}